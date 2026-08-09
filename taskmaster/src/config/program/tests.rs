#[cfg(test)]
mod tests {
    use crate::config::program::{AutoRestart, CommandError};
    use crate::config::{Config, program::Command, program::ProgramConfig};
    use crate::process_handler::OutputFile;
    use libc::unistd::mode_t;
    use signal::Signal;
    use std::collections::HashMap;
    use std::io::Cursor;
    use std::str::FromStr;
    use std::sync::Arc;

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
        pub command: Command,
        pub name: String,
        pub umask: mode_t,
        pub exit_codes: Vec<u8>,
        pub num_procs: u8,
        pub working_dir: String,
        pub auto_restart: AutoRestart,
        pub auto_start: bool,
        pub start_retries: u32,
        pub start_time: u32,
        pub stop_time: u32,
        pub stop_signal: Signal,
        pub clear_env: bool,
        pub stdout: Arc<OutputFile>,
        pub stderr: Arc<OutputFile>,
        pub env: HashMap<String, String>,
    }

    impl TestProgramBuilder {
        fn new(command_string: &str) -> Result<Self, CommandError> {
            Ok(Self {
                command: Command::from_str(command_string)?,
                name: "taskmaster_test_program".to_string(),
                umask: 0o022,
                exit_codes: vec![0],
                num_procs: 1,
                working_dir: "/".to_string(),
                auto_restart: AutoRestart::False,
                auto_start: false,
                start_retries: 0,
                start_time: 0,
                stop_time: 0,
                stop_signal: Signal::SIGTERM,
                clear_env: false,
                stdout: Arc::new(OutputFile::None),
                stderr: Arc::new(OutputFile::None),
                env: HashMap::new(),
            })
        }

        fn build(self) -> Result<ProgramConfig, CommandError> {
            if self.command.exec.is_empty() {
                return Err(CommandError::EmptyCommand);
            }

            let program = ProgramConfig {
                name: self.name,
                cmd: self.command,
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

            Ok(program)
        }
    }

    fn assert_config_parses_to(yaml_content: &str, expected_program: ProgramConfig) {
        let mut programs = HashMap::new();
        programs.insert(expected_program.name.clone(), Arc::new(expected_program));
        let expected_config = Config { programs };

        let config_reader = Cursor::new(yaml_content);
        let parsed_config = Config::from_reader(config_reader);
        assert_eq!(expected_config, parsed_config.expect("error while parsing"));
    }

    fn assert_config_parsing_error(yaml_content: &str) {
        let config_reader = Cursor::new(yaml_content);
        let parsed_config = Config::from_reader(config_reader);
        assert!(parsed_config.is_err());
    }

//     #[test]
//     fn create_yaml_test() {
//         let left = r#"programs:
//         taskmaster_test_program:
//             cmd: "bash -c 'echo Hello $STARTED_BY!'""#;
//         let right = yaml_from_string_command(r#"bash -c 'echo Hello $STARTED_BY!'"#);
//         assert_eq!(left, right)
//     }

//     #[test]
//     fn parsing_default() {
//         let program = TestProgramBuilder::new(r#"bash -c 'echo Hello $STARTED_BY!'"#)
//             .expect("Failed to create builder")
//             .build()
//             .expect("Failed to build program");
//         let yaml_content = yaml_from_string_command(r#"bash -c 'echo Hello $STARTED_BY!'"#);
//         assert_config_parses_to(&yaml_content, program);
//     }

//     #[test]
//     fn parsing_with_multiple_args() {
//         let program = TestProgramBuilder::new(r#"bash -c 'echo test'"#)
//             .expect("Failed to create builder")
//             .build()
//             .expect("Failed to build program");
//         let yaml_content = yaml_from_string_command(r#"bash -c 'echo test'"#);
//         assert_config_parses_to(&yaml_content, program);
//     }

//     #[test]
//     fn parsing_with_env_vars() {
//         let program = TestProgramBuilder::new(r#"echo $HOME"#)
//             .expect("Failed to create builder")
//             .build()
//             .expect("Failed to build program");
//         let yaml_content = yaml_from_string_command(r#"echo $HOME"#);
//         assert_config_parses_to(&yaml_content, program);
//     }

//     #[test]
//     fn parsing_simple_command() {
//         let program = TestProgramBuilder::new(r#"ls -la"#)
//             .expect("Failed to create builder")
//             .build()
//             .expect("Failed to build program");
//         let yaml_content = yaml_from_string_command(r#"ls -la"#);
//         assert_config_parses_to(&yaml_content, program);
//     }

//     #[test]
//     fn parsing_with_umask_octal() {
//         let mut builder = TestProgramBuilder::new("echo test").expect("Failed to create builder");
//         builder.umask = 0o644;
//         let program = builder.build().expect("Failed to build program");
//         let yaml_content = yaml_with_fields(
//             "echo test",
//             r#"
//             umask: "644""#,
//         );
//         assert_config_parses_to(&yaml_content, program);
//     }

//     #[test]
//     fn parsing_with_umask_zero() {
//         let mut builder = TestProgramBuilder::new("echo test").expect("Failed to create builder");
//         builder.umask = 0;
//         let program = builder.build().expect("Failed to build program");
//         let yaml_content = yaml_with_fields(
//             "echo test",
//             r#"
//             umask: "0""#,
//         );
//         assert_config_parses_to(&yaml_content, program);
//     }

//     #[test]
//     fn parsing_program_with_digits_in_name() {
//         let yaml_content = r#"programs:
//         program1:
//             cmd: "echo test""#;
//         assert_config_parsing_error(yaml_content);
//     }

//     #[test]
//     fn parsing_program_with_multiple_digits_not_at_end() {
//         let yaml_content = r#"programs:
//         pro123gram:
//             cmd: "echo test""#;
//         assert_config_parsing_error(yaml_content);
//     }

//     #[test]
//     fn parsing_with_umask_max() {
//         let mut builder = TestProgramBuilder::new("echo test").expect("Failed to create builder");
//         builder.umask = 0o777;
//         let program = builder.build().expect("Failed to build program");
//         let yaml_content = yaml_with_fields(
//             "echo test",
//             r#"
//             umask: "777""#,
//         );
//         assert_config_parses_to(&yaml_content, program);
//     }

//     #[test]
//     fn parsing_with_multiple_fields() {
//         let mut builder = TestProgramBuilder::new("echo test").expect("Failed to create builder");
//         builder.umask = 0o644;
//         builder.working_dir = "/tmp".to_string();
//         builder.auto_start = true;
//         let program = builder.build().expect("Failed to build program");
//         let yaml_content = yaml_with_fields(
//             "echo test",
//             r#"
//             umask: "644"
//             workingdir: "/tmp"
//             autostart: true"#,
//         );
//         assert_config_parses_to(&yaml_content, program);
//     }

//     #[test]
//     fn parsing_with_exit_code_greater_than_256() {
//         let yaml_content = yaml_with_fields(
//             "echo test",
//             r#"
//             exitcodes: [257]"#,
//         );
//         assert_config_parsing_error(&yaml_content);
//     }

//     #[test]
//     fn parsing_with_exit_codes() {
//         let mut builder = TestProgramBuilder::new("echo test").expect("Failed to create builder");
//         builder.exit_codes = vec![0, 1, 2];
//         let program = builder.build().expect("Failed to build program");
//         let yaml_content = yaml_with_fields(
//             "echo test",
//             r#"
//             exitcodes: [0, 1, 2]"#,
//         );
//         assert_config_parses_to(&yaml_content, program);
//     }

//     #[test]
//     fn parsing_with_num_procs() {
//         let mut builder = TestProgramBuilder::new("echo test").expect("Failed to create builder");
//         builder.num_procs = 3;
//         let program = builder.build().expect("Failed to build program");
//         let yaml_content = yaml_with_fields(
//             "echo test",
//             r#"
//             numprocs: 3"#,
//         );
//         assert_config_parses_to(&yaml_content, program);
//     }

//     #[test]
//     fn parsing_with_start_retries() {
//         let mut builder = TestProgramBuilder::new("echo test").expect("Failed to create builder");
//         builder.start_retries = 5;
//         let program = builder.build().expect("Failed to build program");
//         let yaml_content = yaml_with_fields(
//             "echo test",
//             r#"
//             startretries: 5"#,
//         );
//         assert_config_parses_to(&yaml_content, program);
//     }

//     #[test]
//     fn parsing_with_start_time() {
//         let mut builder = TestProgramBuilder::new("echo test").expect("Failed to create builder");
//         builder.start_time = 10;
//         let program = builder.build().expect("Failed to build program");
//         let yaml_content = yaml_with_fields(
//             "echo test",
//             r#"
//             starttime: 10"#,
//         );
//         assert_config_parses_to(&yaml_content, program);
//     }

//     #[test]
//     fn parsing_with_stop_time() {
//         let mut builder = TestProgramBuilder::new("echo test").expect("Failed to create builder");
//         builder.stop_time = 15;
//         let program = builder.build().expect("Failed to build program");
//         let yaml_content = yaml_with_fields(
//             "echo test",
//             r#"
//             stoptime: 15"#,
//         );
//         assert_config_parses_to(&yaml_content, program);
//     }

//     #[test]
//     fn parsing_with_stop_signal_no_sig() {
//         let mut builder = TestProgramBuilder::new("echo test").expect("Failed to create builder");
//         builder.stop_signal = Signal::SIGTERM;
//         let program = builder.build().expect("Failed to build program");
//         let yaml_content = yaml_with_fields(
//             "echo test",
//             r#"
//             stopsignal: "TERM""#,
//         );
//         assert_config_parses_to(&yaml_content, program);
//     }

//     #[test]
//     fn parsing_with_stop_signal_with_sig() {
//         let mut builder = TestProgramBuilder::new("echo test").expect("Failed to create builder");
//         builder.stop_signal = Signal::SIGTERM;
//         let program = builder.build().expect("Failed to build program");
//         let yaml_content = yaml_with_fields(
//             "echo test",
//             r#"
//             stopsignal: "SIGTERM""#,
//         );
//         assert_config_parses_to(&yaml_content, program);
//     }

//     #[test]
//     fn parsing_with_auto_restart() {
//         let mut builder = TestProgramBuilder::new("echo test").expect("Failed to create builder");
//         builder.auto_restart = AutoRestart::True;
//         let program = builder.build().expect("Failed to build program");
//         let yaml_content = yaml_with_fields(
//             "echo test",
//             r#"
//             autorestart: true"#,
//         );
//         assert_config_parses_to(&yaml_content, program);
//     }

//     #[test]
//     fn parsing_with_clear_env() {
//         let mut builder = TestProgramBuilder::new("echo test").expect("Failed to create builder");
//         builder.clear_env = true;
//         let program = builder.build().expect("Failed to build program");
//         let yaml_content = yaml_with_fields(
//             "echo test",
//             r#"
//             clearenv: true"#,
//         );
//         assert_config_parses_to(&yaml_content, program);
//     }

//     #[test]
//     fn parsing_with_stdout() {
//         let mut builder = TestProgramBuilder::new("echo test").expect("Failed to create builder");
//         builder.stdout = Arc::new(
//             OutputFile::new_stdout("/tmp/stdout.log")
//                 .expect("Failed to open stderr file (/tmp/stderr.log)"),
//         );
//         let program = builder.build().expect("Failed to build program");
//         let yaml_content = yaml_with_fields(
//             "echo test",
//             r#"
//             stdout: "/tmp/stdout.log""#,
//         );
//         assert_config_parses_to(&yaml_content, program);
//     }

//     #[test]
//     fn parsing_with_stderr() {
//         let mut builder = TestProgramBuilder::new("echo test").expect("Failed to create builder");
//         builder.stderr = Arc::new(
//             OutputFile::new_stderr("/tmp/stderr.log")
//                 .expect("Failed to open stderr file (/tmp/stderr.log)"),
//         );
//         let program = builder.build().expect("Failed to build program");
//         let yaml_content = yaml_with_fields(
//             "echo test",
//             r#"
//             stderr: "/tmp/stderr.log""#,
//         );
//         assert_config_parses_to(&yaml_content, program);
//     }

//     #[test]
//     fn parsing_with_env() {
//         let mut builder = TestProgramBuilder::new("echo test").expect("Failed to create builder");
//         builder.env.insert("VAR1".to_string(), "value1".to_string());
//         builder.env.insert("VAR2".to_string(), "value2".to_string());
//         let program = builder.build().expect("Failed to build program");
//         let yaml_content = yaml_with_fields(
//             "echo test",
//             r#"
//             env:
//                 VAR1: "value1"
//                 VAR2: "value2""#,
//         );
//         assert_config_parses_to(&yaml_content, program);
//     }
// }
