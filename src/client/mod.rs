mod handle_command;
pub use handle_command::Error;

mod shell;

use std::time::Duration;

use tokio::time::sleep;

use crate::{
    client::handle_command::handle_command,
    rpc::{self, ClientHandle},
};

pub fn start_client(maybe_rl: Option<rustyline::DefaultEditor>) -> Result<(), crate::Error> {
    tokio::runtime::Runtime::new()
        .expect("Failed to init tokio runtime")
        .block_on(run(maybe_rl))
}

async fn run(maybe_rl: Option<rustyline::DefaultEditor>) -> Result<(), crate::Error> {
    let client = connect_client().await?;

    if std::env::args().nth(1).is_some() {
        handle_command(&client, std::env::args().skip(1)).await?;
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
