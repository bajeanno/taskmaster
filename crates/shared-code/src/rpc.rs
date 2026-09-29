use std::{marker::PhantomData, sync::Arc};

use tokio::net::{UnixListener, UnixStream};

const MAX_FRAME_SIZE: usize = 1024 * 1024;
const RPC_UNIX_SOCKET_PATH: &str = "/var/run/taskmaster.d/taskmaster.sock";

pub type ClientHandle = rpc::ClientHandle<UnixStream>;
pub type Client = rpc::Client<UnixStream>;
pub type ConnectClientError = rpc_genie::client::Error;

pub type ServerHandle = rpc::ServerHandle<UnixListener>;
pub type Server = rpc::Server<UnixStream>;
pub type StartServerError = rpc_genie::server::Error;

pub async fn connect_client() -> Result<ClientHandle, ConnectClientError> {
    rpc_genie::unix_socket::connect_client(
        RPC_UNIX_SOCKET_PATH,
        MAX_FRAME_SIZE,
        Arc::new(rpc::Client {
            _stream: PhantomData,
        }),
    )
    .await
}

pub async fn start_server() -> Result<ServerHandle, StartServerError> {
    rpc_genie::unix_socket::start_server(
        RPC_UNIX_SOCKET_PATH,
        MAX_FRAME_SIZE,
        Arc::new(rpc::Server {
            _stream: PhantomData,
        }),
    )
    .await
}

#[rpc_genie::service]
mod rpc {
    pub struct Server<Stream> {}

    pub struct Client<Stream> {}
}
