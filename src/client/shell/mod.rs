use crate::rpc::ClientHandle;
use rustyline::error::ReadlineError;

use crate::client::commands::{parsing::parse_command, send_command};

pub async fn run(
    client: ClientHandle,
    maybe_rl: Option<rustyline::DefaultEditor>,
) -> Result<(), ()> {
    let mut rl = match maybe_rl
        .ok_or(())
        .or_else(|()| rustyline::DefaultEditor::new())
    {
        Ok(rl) => rl,
        Err(err) => {
            eprintln!("Failed to create readline editor: {err}");
            return Err(());
        }
    };

    loop {
        let prompt = match rl.readline("tmcli> ") {
            Ok(line) => {
                // TODO handle error
                let _ = rl.add_history_entry(&line);
                line
            }
            Err(ReadlineError::Interrupted) => {
                continue;
            }
            Err(ReadlineError::Eof) => {
                println!("^D");
                break Ok(());
            }
            Err(err) => {
                eprintln!("Error reading line: {err}");
                break Ok(());
            }
        };
        let iter = prompt.split(' ').map(|item| item.to_string());
        let cmd = match parse_command(iter) {
            Ok(cmd) => {
                let Some(cmd) = cmd else {
                    continue;
                };
                cmd
            }
            Err(e) => {
                eprintln!("{e}");
                continue;
            }
        };
        if let Err(err) = send_command(cmd, &client).await {
            eprintln!("{err}");
        }
    }
}
