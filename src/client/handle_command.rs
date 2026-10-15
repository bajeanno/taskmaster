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
    #[error("Missing argument")]
    MissingArgument,
}

pub async fn handle_command(
    client: &ClientHandle,
    mut args: impl Iterator<Item = String>,
) -> Result<(), Error> {
    match args.next().ok_or(ParseError::EmptyCommand)?.trim() {
        "status" => handle_status_command(client, args).await,
        "reload" => handle_reload_command(client, args).await,
        "start" => handle_start_command(client, args).await,
        "restart" => handle_restart_command(client, args).await,
        "stop" => handle_stop_command(client, args).await,
        "shutdown" => handle_shutdown_command(client, args).await,
        "" => Err(ParseError::EmptyCommand)?,
        other => Err(ParseError::UnknownCommand(other.to_owned()))?,
    }
}

async fn handle_status_command(
    client: &ClientHandle,
    args: impl Iterator<Item = String>,
) -> Result<(), Error> {
    check_no_additional_args(args)?;

    let status = client.server_stub.status().call().await??;
    println!("{status}");
    Ok(())
}

async fn handle_reload_command(
    client: &ClientHandle,
    mut args: impl Iterator<Item = String>,
) -> Result<(), Error> {
    let reload_type = match args.next() {
        None => ReloadType::UseCurrent,
        Some(arg) => match arg.as_str() {
            "--default" => ReloadType::UseDefault,
            "--current" => ReloadType::UseCurrent,
            "--new-default" => {
                let arg = get_next_arg_without_dash(&mut args)?;
                ReloadType::NewDefault(arg)
            }
            _ => {
                let arg = check_doesnt_start_with_dash(arg)?;
                ReloadType::TempConfig(arg)
            }
        },
    };
    check_no_additional_args(args)?;

    client.server_stub.reload(reload_type).call().await??;
    println!("reload");
    Ok(())
}

async fn handle_start_command(
    client: &ClientHandle,
    mut args: impl Iterator<Item = String>,
) -> Result<(), Error> {
    let program_name = get_next_arg_without_dash(&mut args)?;
    check_no_additional_args(args)?;

    client
        .server_stub
        .start(program_name.clone())
        .call()
        .await??;
    println!("{program_name} started");
    Ok(())
}

async fn handle_restart_command(
    client: &ClientHandle,
    mut args: impl Iterator<Item = String>,
) -> Result<(), Error> {
    let program_name = get_next_arg_without_dash(&mut args)?;
    check_no_additional_args(args)?;

    client
        .server_stub
        .restart(program_name.clone())
        .call()
        .await??;
    println!("{program_name} restarted");
    Ok(())
}

async fn handle_stop_command(
    client: &ClientHandle,
    mut args: impl Iterator<Item = String>,
) -> Result<(), Error> {
    let arg = get_next_arg(&mut args)?;

    match arg.as_str() {
        "--all" => {
            check_no_additional_args(args)?;

            client.server_stub.stop_all_processes().call().await??;
            println!("stopped all programs");
        }
        _ => {
            let program_name = check_doesnt_start_with_dash(arg)?;
            check_no_additional_args(args)?;

            client
                .server_stub
                .stop(program_name.clone())
                .call()
                .await??;
            println!("{program_name} stopped");
        }
    }

    Ok(())
}

async fn handle_shutdown_command(
    client: &ClientHandle,
    args: impl Iterator<Item = String>,
) -> Result<(), Error> {
    check_no_additional_args(args)?;

    client.server_stub.shutdown().call().await??;
    println!("shutdown");
    // TODO Need to stop the client
    Ok(())
}

fn get_next_arg(args: &mut impl Iterator<Item = String>) -> Result<String, ParseError> {
    args.next().ok_or(ParseError::MissingArgument)
}

fn check_no_additional_args(mut args: impl Iterator<Item = String>) -> Result<(), ParseError> {
    match args.next() {
        Some(arg) => Err(ParseError::UnexpectedArg(arg)),
        None => Ok(()),
    }
}

fn get_next_arg_without_dash(
    args: &mut impl Iterator<Item = String>,
) -> Result<String, ParseError> {
    let arg = get_next_arg(args)?;
    check_doesnt_start_with_dash(arg)
}

fn check_doesnt_start_with_dash(s: String) -> Result<String, ParseError> {
    if s.starts_with("-") {
        return Err(ParseError::UnexpectedArg(s));
    }
    Ok(s)
}
