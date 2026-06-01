use crate::ProgramConfig;
use tokio::process::Command;

pub(super) fn create_command(config: &ProgramConfig) -> Command {
    let mut command = Command::new("bash");
    command.arg("-c");
    command.arg(config.cmd.clone());

    if *config.clear_env() {
        command.env_clear();
    }
    config.env().iter().for_each(|(key, val)| {
        command.env(key, val);
    });

    command
}
