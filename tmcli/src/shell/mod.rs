use rustyline::{Editor, error::ReadlineError};

use crate::{
    commands::{parsing::parse_command, send_command},
    session::Session,
};

pub async fn run(session: Session) -> Result<(), ()> {
    let mut rl = Editor::<()>::new();
    loop {
        let prompt = match rl.readline("tmcli> ") {
            Ok(line) => {
                rl.add_history_entry(line.as_str());
                line.clone()
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
        if let Err(err) = send_command(cmd, &session).await {
            eprintln!("{err}");
        }
    }
}
