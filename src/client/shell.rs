use crate::{
    client::handle_command::{self, ParseError, handle_command},
    rpc::ClientHandle,
};
use logging::LogLevel;
use rustyline::error::ReadlineError;

pub async fn run(
    client: ClientHandle,
    maybe_rl: Option<rustyline::DefaultEditor>,
) -> Result<(), crate::Error> {
    let mut rl = init_rustyline_editor(maybe_rl)?;

    loop {
        let Some((rl_tmp, line)) = tokio::task::spawn_blocking(|| read_line(rl))
            .await
            .expect("blocking task panicked")?
        else {
            println!("^D");
            break Ok(());
        };
        rl = rl_tmp;

        let command = match shell_words::split(&line) {
            Ok(command) => command,
            Err(err) => {
                logging::log_with_msg("Failed to parse command", &err, LogLevel::Stderr);
                continue;
            }
        };

        match handle_command(&client, command.into_iter()).await {
            Ok(()) => {}
            Err(handle_command::Error::Parsing(ParseError::EmptyCommand)) => continue,
            Err(err) => {
                logging::log_with_msg("Command Failed", &err, LogLevel::Stderr);
                continue;
            }
        }
    }
}

fn init_rustyline_editor(
    maybe_rl: Option<rustyline::DefaultEditor>,
) -> Result<rustyline::DefaultEditor, crate::Error> {
    match maybe_rl {
        Some(rl) => Ok(rl),
        None => rustyline::DefaultEditor::new().map_err(crate::Error::InitCliEditor),
    }
}

fn read_line(
    mut rl: rustyline::DefaultEditor,
) -> Result<Option<(rustyline::DefaultEditor, String)>, crate::Error> {
    loop {
        match rl.readline("tmcli> ") {
            Ok(line) => {
                if let Err(err) = rl.add_history_entry(&line) {
                    logging::log_with_msg(
                        "Failed to add command to history",
                        &err,
                        logging::LogLevel::Stderr,
                    );
                }
                break Ok(Some((rl, line)));
            }
            Err(ReadlineError::Interrupted) => {
                continue;
            }
            Err(ReadlineError::Eof) => {
                break Ok(None);
            }
            Err(err) => {
                break Err(crate::Error::ReadingLine(err));
            }
        }
    }
}
