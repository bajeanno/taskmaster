#[allow(clippy::enum_variant_names)]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("-f/--force is present twice")]
    ForcePresentTwice,
    #[error("-y is present twice")]
    AcknowledgePresentTwice,
    #[error("-d/--daemon-only is present twice")]
    DaemonOnlyPresentTwice,
    #[error("Unexpected argument: {0}")]
    UnexpectedArgument(String),
}

#[derive(Debug, Clone, Copy)]
pub enum StartMode {
    Force { should_start_client: bool },
    Acknowledge { should_start_client: bool },
    Classic,
}

impl StartMode {
    pub fn from_args(args: &mut Vec<String>) -> Result<Self, Error> {
        let mut res = StartMode::Classic;

        for _ in 0..2 {
            let Some(first) = args.first() else { break };

            if first == "--force" || first == "-f" {
                res.set_to_force()?;
            } else if first == "-y" {
                res.set_to_acknowledge()?;
            } else if first == "--daemon-only" || first == "-d" {
                res.set_to_daemon_only()?;
            } else {
                break;
            }

            args.remove(0);
        }

        if let Some(first) = args.first()
            && !res.should_start_client()
        {
            return Err(Error::UnexpectedArgument(first.to_owned()));
        }

        Ok(res)
    }

    fn set_to_force(&mut self) -> Result<(), Error> {
        match self {
            Self::Force { .. } => return Err(Error::ForcePresentTwice),
            Self::Classic => {
                *self = Self::Force {
                    should_start_client: true,
                };
            }
            Self::Acknowledge {
                should_start_client,
            } => {
                *self = Self::Force {
                    should_start_client: *should_start_client,
                };
            }
        }

        Ok(())
    }

    fn set_to_acknowledge(&mut self) -> Result<(), Error> {
        match self {
            Self::Force { .. } => {
                // force gets priority over -y so we don't update the value
            }
            Self::Acknowledge { .. } => return Err(Error::AcknowledgePresentTwice),
            Self::Classic => {
                *self = Self::Acknowledge {
                    should_start_client: true,
                };
            }
        }

        Ok(())
    }

    fn set_to_daemon_only(&mut self) -> Result<(), Error> {
        if !self.should_start_client() {
            return Err(Error::DaemonOnlyPresentTwice);
        }

        *self = match self {
            Self::Classic => Self::Acknowledge {
                should_start_client: false,
            },
            Self::Acknowledge { .. } => Self::Acknowledge {
                should_start_client: false,
            },
            Self::Force { .. } => Self::Force {
                should_start_client: false,
            },
        };

        Ok(())
    }

    pub fn should_start_client(&self) -> bool {
        match self {
            StartMode::Force {
                should_start_client,
            }
            | StartMode::Acknowledge {
                should_start_client,
            } => *should_start_client,

            StartMode::Classic => true,
        }
    }

    pub fn is_force(&self) -> bool {
        matches!(self, Self::Force { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::assert_matches;

    #[test]
    fn force_is_selected_by_both_aliases() {
        for flag in ["--force", "-f"] {
            let mut args = vec![flag.to_string()];
            let start_options = StartMode::from_args(&mut args).unwrap();

            assert_matches!(
                start_options,
                StartMode::Force {
                    should_start_client: true
                }
            );
            assert!(start_options.is_force());
            assert!(args.is_empty());
        }
    }

    #[test]
    fn force_takes_priority_over_acknowledge() {
        for mut args in [
            vec!["--force".to_string(), "-y".to_string()],
            vec!["-y".to_string(), "-f".to_string()],
        ] {
            let start_options = StartMode::from_args(&mut args).unwrap();

            assert_matches!(
                start_options,
                StartMode::Force {
                    should_start_client: true
                }
            );
            assert!(args.is_empty());
        }
    }

    #[test]
    fn acknowledge_is_selected_by_y() {
        let mut args = vec!["-y".to_string()];
        let start_options = StartMode::from_args(&mut args).unwrap();

        assert_matches!(
            start_options,
            StartMode::Acknowledge {
                should_start_client: true
            }
        );
        assert!(!start_options.is_force());
        assert!(args.is_empty());
    }

    #[test]
    fn classic_is_selected_without_a_start_mode_flag() {
        for mut args in [
            vec![] as Vec<String>,
            vec!["".to_string()],
            vec!["--y".to_string()],
            vec!["force".to_string()],
        ] {
            let original_args = args.clone();
            let start_options = StartMode::from_args(&mut args).unwrap();

            assert_matches!(start_options, StartMode::Classic);
            assert!(!start_options.is_force());
            assert!(start_options.should_start_client());
            assert_eq!(args, original_args);
        }
    }

    #[test]
    fn daemon_only_is_selected_by_both_aliases() {
        for flag in ["--daemon-only", "-d"] {
            let mut args = vec![flag.to_string()];
            let start_options = StartMode::from_args(&mut args).unwrap();

            assert_matches!(
                start_options,
                StartMode::Acknowledge {
                    should_start_client: false
                }
            );
            assert!(args.is_empty());
        }
    }

    #[test]
    fn daemon_only_can_be_combined_with_start_mode() {
        let mut args = vec!["-y".to_string(), "-d".to_string()];
        let start_options = StartMode::from_args(&mut args).unwrap();

        assert_matches!(
            start_options,
            StartMode::Acknowledge {
                should_start_client: false
            }
        );
        assert!(args.is_empty());
    }

    #[test]
    fn duplicate_flags_are_rejected() {
        let mut force_args = vec!["-f".to_string(), "--force".to_string()];
        assert!(matches!(
            StartMode::from_args(&mut force_args),
            Err(Error::ForcePresentTwice)
        ));

        let mut acknowledge_args = vec!["-y".to_string(), "-y".to_string()];
        assert!(matches!(
            StartMode::from_args(&mut acknowledge_args),
            Err(Error::AcknowledgePresentTwice)
        ));

        let mut daemon_only_args = vec!["-d".to_string(), "--daemon-only".to_string()];
        assert!(matches!(
            StartMode::from_args(&mut daemon_only_args),
            Err(Error::DaemonOnlyPresentTwice)
        ));
    }

    #[test]
    fn daemon_only_rejects_unexpected_arguments() {
        for mut args in [
            vec!["-d".to_string(), "command".to_string()],
            vec!["-y".to_string(), "-d".to_string(), "command".to_string()],
        ] {
            assert_matches!(
                StartMode::from_args(&mut args),
                Err(Error::UnexpectedArgument(argument)) if argument == "command"
            );
        }
    }

    #[test]
    fn regular_modes_leave_client_arguments_unconsumed() {
        let mut args = vec!["-y".to_string(), "command".to_string()];
        let start_options = StartMode::from_args(&mut args).unwrap();

        assert_matches!(
            start_options,
            StartMode::Acknowledge {
                should_start_client: true
            }
        );
        assert_eq!(args, vec!["command".to_string()]);
    }
}
