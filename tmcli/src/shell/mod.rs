mod error;

use crate::{
    commands::{parsing::parse_command, send_command},
    session::Session,
    shell::error::ShellError,
};

pub async fn run(session: Session) -> Result<(), ShellError> {
    loop {
        let mut prompt = String::new();
        std::io::stdin()
            .read_line(&mut prompt)
            .map_err(ShellError::ReadingStdin)?;
        let iter = prompt.split(' ').map(|item| item.to_string());
        let cmd = match parse_command(iter) {
            Ok(cmd) => {
                let Some(cmd) = cmd else {
                    return Ok(());
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
