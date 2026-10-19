mod handle_command;
pub use handle_command::Error;

mod shell;

use std::{sync::PoisonError, time::Duration};

use tokio::time::sleep;

use crate::{
    ARGS,
    client::handle_command::handle_command,
    rpc::{self, ClientHandle},
};

pub fn run(maybe_rl: Option<rustyline::DefaultEditor>) -> Result<(), crate::Error> {
    tokio::runtime::Runtime::new()
        .expect("Failed to init tokio runtime")
        .block_on(start_client(maybe_rl))
}

async fn start_client(maybe_rl: Option<rustyline::DefaultEditor>) -> Result<(), crate::Error> {
    let client = connect_client().await?;

    let args = ARGS.lock().unwrap_or_else(PoisonError::into_inner).clone();

    if args.get(1).is_some() {
        handle_command(&client, args.into_iter().skip(1)).await?;
    } else {
        shell::run(client, maybe_rl).await?;
    }

    Ok(())
}

async fn connect_client() -> Result<ClientHandle, crate::Error> {
    let mut i = 0;
    loop {
        match rpc::connect_client().await {
            Ok(client) => return Ok(client),
            Err(err) => {
                if i >= 4 {
                    return Err(err.into());
                }

                i += 1;
                sleep(Duration::from_millis(400)).await;
            }
        }
    }
}
