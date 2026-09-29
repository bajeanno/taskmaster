use rustyline::{Editor, error::ReadlineError};
use shared_code::rpc::ClientHandle;

use crate::commands::{parsing::parse_command, send_command};

pub async fn run(client: ClientHandle) -> Result<(), ()> {
    let mut rl = Editor::<()>::new();
    loop {
        let prompt = match rl.readline("tmcli> ") {
            Ok(line) => {
                rl.add_history_entry(&line);
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
