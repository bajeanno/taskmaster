use std::{marker::PhantomData, sync::Arc};

use tokio::net::{UnixListener, UnixStream};

use crate::daemon::TasksManagerHandle;

const MAX_FRAME_SIZE: usize = 1024 * 1024;
pub const RPC_UNIX_SOCKET_PATH: &str = "/var/run/taskmaster.d/taskmaster.sock";

pub type ClientHandle = service::ClientHandle<UnixStream>;
pub type ConnectClientError = rpc_genie::client::Error;

pub type ServerHandle = service::ServerHandle<UnixListener>;
pub type StartServerError = rpc_genie::server::Error;

pub async fn connect_client() -> Result<ClientHandle, ConnectClientError> {
    rpc_genie::unix_socket::connect_client(
        RPC_UNIX_SOCKET_PATH,
        MAX_FRAME_SIZE,
        Arc::new(service::Client {
            _stream: PhantomData,
        }),
    )
    .await
}

pub async fn start_server(
    tasks_manager_handle: Arc<TasksManagerHandle>,
) -> Result<ServerHandle, StartServerError> {
    rpc_genie::unix_socket::start_server(
        RPC_UNIX_SOCKET_PATH,
        MAX_FRAME_SIZE,
        Arc::new(service::Server {
            tasks_manager_handle,
            _stream: PhantomData,
        }),
    )
    .await
}

#[rpc_genie::service]
mod service {
    use crate::daemon::{ReloadType, ServerCommandError, TasksManagerHandle, TasksManagerStatus};
    use std::sync::Arc;

    pub struct Server<Stream> {
        pub(super) tasks_manager_handle: Arc<TasksManagerHandle>,
    }

    type Result<T, Err = ServerCommandError> = core::result::Result<T, Err>;

    impl<Stream> Server<Stream> {
        #[remote_method]
        pub async fn status(&self) -> Result<TasksManagerStatus> {
            self.tasks_manager_handle.status().await
        }

        #[remote_method]
        pub async fn reload(&self, reload_type: ReloadType) -> Result<()> {
            self.tasks_manager_handle.reload(reload_type).await
        }

        #[remote_method]
        pub async fn start(&self, program_name: String) -> Result<()> {
            self.tasks_manager_handle.start(program_name).await
        }

        #[remote_method]
        pub async fn restart(&self, program_name: String) -> Result<()> {
            self.tasks_manager_handle.restart(program_name).await
        }

        #[remote_method]
        pub async fn stop(&self, program_name: String) -> Result<()> {
            self.tasks_manager_handle.stop(program_name).await
        }

        #[remote_method]
        pub async fn stop_all_processes(&self) -> Result<()> {
            self.tasks_manager_handle.stop_all_processes().await
        }

        #[remote_method]
        pub async fn restore_config_file(&self) -> Result<()> {
            self.tasks_manager_handle.restore_config_file().await
        }

        #[remote_method]
        pub async fn shutdown(&self) -> Result<()> {
            self.tasks_manager_handle.shutdown().await
        }
    }

    pub struct Client<Stream> {}
}
