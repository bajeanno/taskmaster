use crate::CommandSender;

#[allow(dead_code)]
pub struct Handle {
    command_sender: CommandSender,
}

impl Handle {
    pub(super) fn new(command_sender: CommandSender) -> Handle {
        Handle { command_sender }
    }
}
