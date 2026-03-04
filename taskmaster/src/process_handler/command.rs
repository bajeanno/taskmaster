use crate::Program;
use tokio::process::Command;

pub(super) fn create_command(config: &Program) -> Command {
    let mut command = Command::new(config.cmd.exec.clone());
    for arg in config.cmd.args.iter() {
        command.arg(arg);
    }

    if *config.clear_env() {
        command.env_clear();
    }
    config.env().iter().for_each(|(key, val)| {
        command.env(key, val);
    });

    command
}
