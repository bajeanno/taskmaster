use crate::{
    daemon::{ReloadType, ServerCommandError},
    rpc::ClientHandle,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Error parsing command")]
    Parsing(#[from] ParseError),
    #[error("RPC request failed")]
    Rpc(#[from] rpc_genie::CallError),
    #[error("Command failed")]
    CommandExecution(#[from] ServerCommandError),
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("Command can not be empty")]
    EmptyCommand,
    #[error("Unknown command: {0}")]
    UnknownCommand(String),
    #[error("UnexpectedArg: {0}")]
    UnexpectedArg(String),
    #[error("{arg_name} may not start with '-': found {found}")]
    InvalidDashArg {
        arg_name: &'static str,
        found: String,
    },
    #[error("Missing argument {0}")]
    MissingArgument(&'static str),
}

pub enum CommandWasShutdown {
    True,
    False,
}

macro_rules! handle_command {
    (
        $args_iter:ident;
        $($cases:tt)*
    ) => {{
        let command_name = $args_iter.next().ok_or(ParseError::EmptyCommand)?;
        let command_name = command_name.trim();

        handle_command_cases!(
            || Err(ParseError::UnknownCommand(command_name.to_owned()).into()),
            $args_iter,
            command_name,
            $($cases)*
        )
    }};
}

macro_rules! handle_command_cases {
    // -- literal => sub_commands! { .. }
    (
        $default:expr,
        $args_iter:ident,
        $command_name:ident,

        #[doc = $doc:literal]
        $command_lit:literal => sub_commands! { $($handler:tt)* },

        $($cases:tt)*
    ) => {
        match $command_name.as_ref() {
            $command_lit => { handle_sub_command!($args_iter, $($handler)*) }
            _ => { handle_command_cases!($default, $args_iter, $command_name, $($cases)*) }
        }
    };

    // -- literal, [ident] => expr
    (
        $default:expr,
        $args_iter:ident,
        $command_name:ident,

        #[doc = $doc:literal]
        $command_lit:literal $(, $command_param:ident)* $(,)? => $handler:expr,

        $($cases:tt)*
    ) => {
        match $command_name.as_ref() {
            $command_lit => {
                $(
                    let $command_param = get_next_arg_without_dash(
                        &mut $args_iter,
                        stringify!($command_param),
                    )?;
                )*
                check_no_additional_args($args_iter)?;
                $handler
            }
            _ => { handle_command_cases!($default, $args_iter, $command_name, $($cases)*) }
        }
    };

    // -- literal => expr
    (
        $default:expr,
        $args_iter:ident,
        $command_name:ident,

        #[doc = $doc:literal]
        $command_lit:literal => $handler:expr,

        $($cases:tt)*
    ) => {
        match $command_name.as_ref() {
            $command_lit => {
                check_no_additional_args($args_iter)?;
                $handler
            }
            _ => { handle_command_cases!($default, $args_iter, $command_name, $($cases)*) }
        }
    };

    // -- [ident] => expr
    (
        $default:expr,
        $args_iter:ident,
        $command_name:ident,

        #[doc = $doc:literal]
        $command_param:ident $(, $additional_command_param:ident)* $(,)? => $handler:expr,
    ) => {{
        let $command_param = check_doesnt_start_with_dash(
            $command_name,
            stringify!($command_param),
        )?;
        $(
            let $additional_command_param = get_next_arg_without_dash(
                &mut $args_iter,
                stringify!($additional_command_param)
            )?;
        )*
        check_no_additional_args($args_iter)?;
        $handler
    }};

    // -- No cases left
    ($default:expr, $args_iter:ident, $command_name:ident, ) => {
        return ($default)()
    };
}

macro_rules! handle_sub_command {
    ($args_iter:ident, $($cases:tt)*) => {{
        let arg = get_next_arg(&mut $args_iter, stringify!($command_param))?;

        handle_command_cases!(
            || Err(ParseError::UnexpectedArg(arg).into()),
            $args_iter,
            arg,
            $($cases)*
        )
    }};
}

pub async fn handle_command(
    client: &ClientHandle,
    mut args: impl Iterator<Item = String>,
) -> Result<CommandWasShutdown, Error> {
    handle_command! {
        args;

        /// Prints the status of the programs handled by taskmaster
        "status" => handle_status_command(client).await?,

        /// Reload config file
        "reload" => sub_commands! {
            /// Reload config file from the default config file
            "--default" => handle_reload_command(client, ReloadType::UseDefault).await?,

            /// Reload config file from the current config file
            "--current" => handle_reload_command(client, ReloadType::UseCurrent).await?,

            /// Reload config file at config_file_path and set it as the new default
            "--new-default", config_file_path => {
                handle_reload_command(client, ReloadType::NewDefault(config_file_path)).await?
            },

            /// Reload config file at config_file_path
            config_file_path => {
                handle_reload_command(client, ReloadType::TempConfig(config_file_path)).await?
            },
        },

        /// Starts the program named program_name
        "start", program_name => handle_start_command(client, program_name).await?,

        /// Restarts the program named program_name
        "restart", program_name => handle_restart_command(client, program_name).await?,

        /// Stops programs
        "stop" => sub_commands! {
            /// Stops all programs handled by taskmaster
            "--all" => handle_stop_all_command(client).await?,

            /// Stops the program named program_name
            program_name => handle_stop_command(client, program_name).await?,
        },

        /// Rewrites the active config to the last used config file
        "restore-config-file" => handle_restore_config_file_command(client).await?,

        /// Stops all programs and shutdowns the daemon
        "shutdown" => {
            handle_shutdown_command(client).await?;
            return Ok(CommandWasShutdown::True);
        },
    }
    Ok(CommandWasShutdown::False)
}

async fn handle_status_command(client: &ClientHandle) -> Result<(), Error> {
    let status = client.server_stub.status().call().await??;
    println!("{status}");
    Ok(())
}

async fn handle_reload_command(
    client: &ClientHandle,
    reload_type: ReloadType,
) -> Result<(), Error> {
    client.server_stub.reload(reload_type).call().await??;
    println!("reload");
    Ok(())
}

async fn handle_start_command(client: &ClientHandle, program_name: String) -> Result<(), Error> {
    client
        .server_stub
        .start(program_name.clone())
        .call()
        .await??;
    println!("{program_name} started");
    Ok(())
}

async fn handle_restart_command(client: &ClientHandle, program_name: String) -> Result<(), Error> {
    client
        .server_stub
        .restart(program_name.clone())
        .call()
        .await??;
    println!("{program_name} restarted");
    Ok(())
}

async fn handle_stop_command(client: &ClientHandle, program_name: String) -> Result<(), Error> {
    client
        .server_stub
        .stop(program_name.clone())
        .call()
        .await??;
    println!("{program_name} stopped");
    Ok(())
}

async fn handle_stop_all_command(client: &ClientHandle) -> Result<(), Error> {
    client.server_stub.stop_all_processes().call().await??;
    println!("stopped all programs");
    Ok(())
}

async fn handle_restore_config_file_command(client: &ClientHandle) -> Result<(), Error> {
    client.server_stub.restore_config_file().call().await??;
    println!("config file restored");
    Ok(())
}

async fn handle_shutdown_command(client: &ClientHandle) -> Result<(), Error> {
    client.server_stub.shutdown().call().await??;
    println!("shutdown");
    Ok(())
}

fn get_next_arg(
    args: &mut impl Iterator<Item = String>,
    arg_name: &'static str,
) -> Result<String, ParseError> {
    args.next().ok_or(ParseError::MissingArgument(arg_name))
}

fn check_no_additional_args(mut args: impl Iterator<Item = String>) -> Result<(), ParseError> {
    match args.next() {
        Some(arg) => Err(ParseError::UnexpectedArg(arg)),
        None => Ok(()),
    }
}

fn get_next_arg_without_dash(
    args: &mut impl Iterator<Item = String>,
    arg_name: &'static str,
) -> Result<String, ParseError> {
    let arg = get_next_arg(args, arg_name)?;
    check_doesnt_start_with_dash(arg, arg_name)
}

fn check_doesnt_start_with_dash(s: String, arg_name: &'static str) -> Result<String, ParseError> {
    if s.starts_with("-") {
        return Err(ParseError::InvalidDashArg { arg_name, found: s });
    }
    Ok(s)
}
