#[allow(unused)]
use crate::config::{
    Config,
    program::{Command, Program},
};
use signal::Signal;
use std::collections::HashMap;
use std::io::Cursor;
use tokio::process::Command as TokioCommand;

#[cfg(test)]
fn yaml_from_string_command(command: &str) -> String {
    let start = r#"programs:
    taskmaster_test_program:
        cmd: ""#;
    String::from(start) + command + "\""
}

#[test]
#[cfg(test)]
fn create_yaml_test() {
    {
        let left = r#"programs:
    taskmaster_test_program:
        cmd: "bash -c 'echo Hello $STARTED_BY!'""#;
        let right = yaml_from_string_command(r#"bash -c 'echo Hello $STARTED_BY!'"#);
        assert_eq!(left, right)
    }
}

#[test]
#[cfg(test)]
fn parsing_default() {
    use crate::config::program::AutoRestart;

    let command_string = r#"bash -c 'echo Hello $STARTED_BY!'"#;
    let parts = shell_words::split(&command_string).expect("bad command in tests");
    let mut parts_iter = parts.iter();
    let cmd = parts_iter.next().expect("empty command in tests");
    let mut program = Program {
        name: "taskmaster_test_program".to_string(),
        cmd: Command {
            command: TokioCommand::new(cmd),
            string: command_string.to_string(),
        },
        pids: vec![],
        umask: 0o666,
        env: HashMap::new(),
        exit_codes: vec![0],
        num_procs: 1,
        working_dir: "/".to_string(),
        auto_restart: AutoRestart::False,
        auto_start: false,
        start_retries: 0,
        start_time: 0,
        stop_time: 0,
        stop_signal: Signal::SIGINT,
        clear_env: false,
        stdout: "/dev/null".to_string(),
        stderr: "/dev/null".to_string(),
    };

    parts_iter.for_each(|arg| {
        program.cmd.command.arg(arg);
    });
    let mut config = Config {
        programs: Vec::new(),
    };
    config.programs.push(program);
    let yaml_content = yaml_from_string_command(command_string);
    let config_reader = Cursor::new(yaml_content);
    let test_config = Config::from_reader(config_reader);
    assert_eq!(config, test_config.expect("error while parsing"));
}
