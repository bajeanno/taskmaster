mod commands;
mod session;
mod shell;

use crate::session::Session;

#[tokio::main]
async fn main() {
    let session = match Session::new().await {
        Ok(session) => session,
        Err(err) => {
            eprintln!("Failed to instanciate connection: {err}");
            // TODO return error code
            return;
        }
    };

    if std::env::args().nth(1).is_some() {
        commands::oneshot_command::run(session)
            .await
            .unwrap_or_else(|err| eprintln!("{err}"));
        // TODO return error code on error
    } else {
        shell::run(session)
            .await
            .unwrap_or_else(|err| eprintln!("{err}"));
        // TODO return error code on error
    }
}
