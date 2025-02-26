pub mod parsing;
mod session;

use commands::{ClientCommand, ServerCommand};
use session::Session;

pub fn send_command(_cmd: ServerCommand) -> ClientCommand {
    let _session = Session::new();
    return ClientCommand::SuccessfulConnection;
}
