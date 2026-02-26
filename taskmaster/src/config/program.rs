use derive_getters::Getters;
use libc::sys::types::Pid;
use serde::{Deserialize, Deserializer, de};
use signal::Signal;
use std::{collections::HashMap, fmt::Display, str::FromStr};

#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Deserialize, Default)]
pub enum AutoRestart {
    #[serde(rename = "true")]
    True,
    #[default]
    #[serde(rename = "false")]
    False,
    #[serde(rename = "unexpected")]
    OnFailure,
}

#[allow(dead_code)] // TODO: remove this
#[derive(Debug, Getters, Deserialize)]
#[cfg_attr(test, derive(PartialEq))]
#[serde(deny_unknown_fields)]
pub struct Program {
    #[serde(skip)]
    name: String,

    #[serde(default)]
    pids: Vec<Pid>,

    #[serde(default = "default_umask", deserialize_with = "deserialize_umask")]
    umask: u32,

    pub cmd: String,

    #[serde(rename = "numprocs", default = "default_num_procs")]
    num_procs: u32,

    #[serde(rename = "workingdir", default = "default_work_dir")]
    working_dir: String,

    #[serde(rename = "autostart", default)]
    auto_start: bool,

    #[serde(rename = "autorestart", default)]
    auto_restart: AutoRestart,

    #[serde(rename = "exitcodes", default = "default_exit_codes")]
    exit_codes: Vec<u8>,

    #[serde(rename = "startretries", default)]
    start_retries: u32,

    #[serde(rename = "starttime", default)]
    start_time: u32,

    #[serde(
        rename = "stopsignal",
        default = "default_signal",
        deserialize_with = "deserialize_signal"
    )]
    stop_signal: Signal,

    #[serde(rename = "stoptime", default)]
    stop_time: u32,

    #[serde(default = "default_output")]
    stdout: String,

    #[serde(default = "default_output")]
    stderr: String,

    #[serde(rename = "clearenv", default)]
    clear_env: bool,

    #[serde(default)]
    env: HashMap<String, String>,
}

fn deserialize_signal<'de, D>(deserializer: D) -> Result<Signal, D::Error>
where
    D: Deserializer<'de>,
{
    let signal: Signal = Signal::from_str(
        String::deserialize(deserializer)
            .map_err(|err| serde::de::Error::custom(format!("Failed to parse signal: {err}")))?
            .as_str(),
    )
    .map_err(|err| de::Error::custom(format!("Failed to convert signal from string: {err}")))?;
    Ok(signal)
}

fn deserialize_umask<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: Deserializer<'de>,
{
    let umask_str = String::deserialize(deserializer)
        .map_err(|err| serde::de::Error::custom(format!("Failed to parse umask: {err}")))?;
    let umask = u32::from_str_radix(umask_str.as_str(), 8).map_err(|err| {
        serde::de::Error::custom(format!("ParseIntError on umask parsing: {err}"))
    })?;
    if umask > 0o777 {
        Err(serde::de::Error::custom(
            "umask is greater than 0o777 (max value accepted)",
        ))
    } else {
        Ok(umask)
    }
}

fn default_output() -> String {
    "/dev/null".to_string()
}

fn default_signal() -> Signal {
    Signal::SIGINT
}

fn default_num_procs() -> u32 {
    1
}

fn default_work_dir() -> String {
    String::from("/")
}

fn default_exit_codes() -> Vec<u8> {
    vec![0]
}

fn default_umask() -> u32 {
    0o666
}

impl Display for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:<15}{:50}{: ^15?}{:>10o}",
            self.name, self.cmd, self.pids, self.umask,
        )
    }
}

impl Program {
    pub(super) fn name_mut(&mut self) -> &mut String {
        &mut self.name
    }
}

#[cfg(test)]
mod tests {
    use crate::config::program::AutoRestart;
    use crate::config::{Config, program::Program};
    use signal::Signal;
    use std::collections::HashMap;
    use std::io::Cursor;

    fn yaml_from_string_command(command: &str) -> String {
        let start = r#"programs:
        taskmaster_test_program:
            cmd: ""#;
        String::from(start) + command + "\""
    }

    fn yaml_with_fields(command: &str, additional_fields: &str) -> String {
        let start = r#"programs:
        taskmaster_test_program:
            cmd: ""#;
        String::from(start) + command + "\"" + additional_fields
    }

    pub struct TestProgramBuilder {
        pub command_string: String,
        pub name: String,
        pub umask: u32,
        pub exit_codes: Vec<u8>,
        pub num_procs: u32,
        pub working_dir: String,
        pub auto_restart: AutoRestart,
        pub auto_start: bool,
        pub start_retries: u32,
        pub start_time: u32,
        pub stop_time: u32,
        pub stop_signal: Signal,
        pub clear_env: bool,
        pub stdout: String,
        pub stderr: String,
        pub env: HashMap<String, String>,
    }

    impl TestProgramBuilder {
        fn new(command_string: &str) -> Self {
            Self {
                command_string: command_string.to_string(),
                name: "taskmaster_test_program".to_string(),
                umask: 0o666,
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
                env: HashMap::new(),
            }
        }

        fn build(self) -> Program {
            if self.command_string.is_empty() {
                panic!("Empty command in program")
            }

            let program = Program {
                name: self.name,
                cmd: self.command_string,
                pids: vec![],
                umask: self.umask,
                env: self.env,
                exit_codes: self.exit_codes,
                num_procs: self.num_procs,
                working_dir: self.working_dir,
                auto_restart: self.auto_restart,
                auto_start: self.auto_start,
                start_retries: self.start_retries,
                start_time: self.start_time,
                stop_time: self.stop_time,
                stop_signal: self.stop_signal,
                clear_env: self.clear_env,
                stdout: self.stdout,
                stderr: self.stderr,
            };

            program
        }
    }

    fn assert_config_parses_to(yaml_content: &str, expected_program: Program) {
        let expected_config = Config {
            programs: vec![expected_program],
        };

        let config_reader = Cursor::new(yaml_content);
        let parsed_config = Config::from_reader(config_reader);
        assert_eq!(expected_config, parsed_config.expect("error while parsing"));
    }

    fn assert_config_parsing_error(yaml_content: &str) {
        let config_reader = Cursor::new(yaml_content);
        let parsed_config = Config::from_reader(config_reader);
        assert!(parsed_config.is_err());
    }

    #[test]
    fn create_yaml_test() {
        let left = r#"programs:
        taskmaster_test_program:
            cmd: "bash -c 'echo Hello $STARTED_BY!'""#;
        let right = yaml_from_string_command(r#"bash -c 'echo Hello $STARTED_BY!'"#);
        assert_eq!(left, right)
    }

    #[test]
    fn parsing_default() {
        let program = TestProgramBuilder::new(r#"bash -c 'echo Hello $STARTED_BY!'"#).build();
        let yaml_content = yaml_from_string_command(r#"bash -c 'echo Hello $STARTED_BY!'"#);
        assert_config_parses_to(&yaml_content, program);
    }

    #[test]
    fn parsing_with_multiple_args() {
        let program = TestProgramBuilder::new(r#"bash -c 'echo test'"#).build();
        let yaml_content = yaml_from_string_command(r#"bash -c 'echo test'"#);
        assert_config_parses_to(&yaml_content, program);
    }

    #[test]
    fn parsing_with_env_vars() {
        let program = TestProgramBuilder::new(r#"echo $HOME"#).build();
        let yaml_content = yaml_from_string_command(r#"echo $HOME"#);
        assert_config_parses_to(&yaml_content, program);
    }

    #[test]
    fn parsing_simple_command() {
        let program = TestProgramBuilder::new(r#"ls -la"#).build();
        let yaml_content = yaml_from_string_command(r#"ls -la"#);
        assert_config_parses_to(&yaml_content, program);
    }

    #[test]
    fn parsing_with_umask_octal() {
        let mut builder = TestProgramBuilder::new("echo test");
        builder.umask = 0o644;
        let program = builder.build();
        let yaml_content = yaml_with_fields(
            "echo test",
            r#"
            umask: "644""#,
        );
        assert_config_parses_to(&yaml_content, program);
    }

    #[test]
    fn parsing_with_umask_zero() {
        let mut builder = TestProgramBuilder::new("echo test");
        builder.umask = 0;
        let program = builder.build();
        let yaml_content = yaml_with_fields(
            "echo test",
            r#"
            umask: "0""#,
        );
        assert_config_parses_to(&yaml_content, program);
    }

    #[test]
    fn parsing_with_umask_max() {
        let mut builder = TestProgramBuilder::new("echo test");
        builder.umask = 0o777;
        let program = builder.build();
        let yaml_content = yaml_with_fields(
            "echo test",
            r#"
            umask: "777""#,
        );
        assert_config_parses_to(&yaml_content, program);
    }

    #[test]
    fn parsing_with_multiple_fields() {
        let mut builder = TestProgramBuilder::new("echo test");
        builder.umask = 0o644;
        builder.working_dir = "/tmp".to_string();
        builder.auto_start = true;
        let program = builder.build();
        let yaml_content = yaml_with_fields(
            "echo test",
            r#"
            umask: "644"
            workingdir: "/tmp"
            autostart: true"#,
        );
        assert_config_parses_to(&yaml_content, program);
    }

    #[test]
    fn parsing_with_exit_code_greater_than_256() {
        let yaml_content = yaml_with_fields(
            "echo test",
            r#"
            exitcodes: [257]"#,
        );
        assert_config_parsing_error(&yaml_content);
    }

    #[test]
    fn parsing_with_exit_codes() {
        let mut builder = TestProgramBuilder::new("echo test");
        builder.exit_codes = vec![0, 1, 2];
        let program = builder.build();
        let yaml_content = yaml_with_fields(
            "echo test",
            r#"
            exitcodes: [0, 1, 2]"#,
        );
        assert_config_parses_to(&yaml_content, program);
    }

    #[test]
    fn parsing_with_num_procs() {
        let mut builder = TestProgramBuilder::new("echo test");
        builder.num_procs = 3;
        let program = builder.build();
        let yaml_content = yaml_with_fields(
            "echo test",
            r#"
            numprocs: 3"#,
        );
        assert_config_parses_to(&yaml_content, program);
    }

    #[test]
    fn parsing_with_start_retries() {
        let mut builder = TestProgramBuilder::new("echo test");
        builder.start_retries = 5;
        let program = builder.build();
        let yaml_content = yaml_with_fields(
            "echo test",
            r#"
            startretries: 5"#,
        );
        assert_config_parses_to(&yaml_content, program);
    }

    #[test]
    fn parsing_with_start_time() {
        let mut builder = TestProgramBuilder::new("echo test");
        builder.start_time = 10;
        let program = builder.build();
        let yaml_content = yaml_with_fields(
            "echo test",
            r#"
            starttime: 10"#,
        );
        assert_config_parses_to(&yaml_content, program);
    }

    #[test]
    fn parsing_with_stop_time() {
        let mut builder = TestProgramBuilder::new("echo test");
        builder.stop_time = 15;
        let program = builder.build();
        let yaml_content = yaml_with_fields(
            "echo test",
            r#"
            stoptime: 15"#,
        );
        assert_config_parses_to(&yaml_content, program);
    }

    #[test]
    fn parsing_with_stop_signal() {
        let mut builder = TestProgramBuilder::new("echo test");
        builder.stop_signal = Signal::SIGTERM;
        let program = builder.build();
        let yaml_content = yaml_with_fields(
            "echo test",
            r#"
            stopsignal: "SIGTERM""#,
        );
        assert_config_parses_to(&yaml_content, program);
    }

    #[test]
    fn parsing_with_auto_restart() {
        let mut builder = TestProgramBuilder::new("echo test");
        builder.auto_restart = AutoRestart::True;
        let program = builder.build();
        let yaml_content = yaml_with_fields(
            "echo test",
            r#"
            autorestart: true"#,
        );
        assert_config_parses_to(&yaml_content, program);
    }

    #[test]
    fn parsing_with_clear_env() {
        let mut builder = TestProgramBuilder::new("echo test");
        builder.clear_env = true;
        let program = builder.build();
        let yaml_content = yaml_with_fields(
            "echo test",
            r#"
            clearenv: true"#,
        );
        assert_config_parses_to(&yaml_content, program);
    }

    #[test]
    fn parsing_with_stdout() {
        let mut builder = TestProgramBuilder::new("echo test");
        builder.stdout = "/var/log/stdout.log".to_string();
        let program = builder.build();
        let yaml_content = yaml_with_fields(
            "echo test",
            r#"
            stdout: "/var/log/stdout.log""#,
        );
        assert_config_parses_to(&yaml_content, program);
    }

    #[test]
    fn parsing_with_stderr() {
        let mut builder = TestProgramBuilder::new("echo test");
        builder.stderr = "/var/log/stderr.log".to_string();
        let program = builder.build();
        let yaml_content = yaml_with_fields(
            "echo test",
            r#"
            stderr: "/var/log/stderr.log""#,
        );
        assert_config_parses_to(&yaml_content, program);
    }

    #[test]
    fn parsing_with_env() {
        let mut builder = TestProgramBuilder::new("echo test");
        builder.env.insert("VAR1".to_string(), "value1".to_string());
        builder.env.insert("VAR2".to_string(), "value2".to_string());
        let program = builder.build();
        let yaml_content = yaml_with_fields(
            "echo test",
            r#"
            env:
                VAR1: "value1"
                VAR2: "value2""#,
        );
        assert_config_parses_to(&yaml_content, program);
    }
}
