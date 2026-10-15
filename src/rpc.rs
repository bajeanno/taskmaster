use std::{marker::PhantomData, sync::Arc};

use tokio::{
    net::{UnixListener, UnixStream},
    sync::Mutex,
};

use crate::daemon::TasksManagerHandle;

const MAX_FRAME_SIZE: usize = 1024 * 1024;
const RPC_UNIX_SOCKET_PATH: &str = "/var/run/taskmaster.d/taskmaster.sock";

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
    tasks_manager_handle: Arc<Mutex<TasksManagerHandle>>,
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
    use crate::daemon::{
        ReloadType, ServerCommandError, TaskManagerCommand, TasksManagerHandle, TasksManagerStatus,
    };
    use std::sync::Arc;
    use tokio::sync::{Mutex, oneshot};

    pub struct Server<Stream> {
        pub(super) tasks_manager_handle: Arc<Mutex<TasksManagerHandle>>,
    }

    type Result<T, Err = ServerCommandError> = core::result::Result<T, Err>;

    impl<Stream> Server<Stream> {
        #[remote_method]
        pub async fn status(&self) -> Result<TasksManagerStatus> {
            let (sender, receiver) = oneshot::channel();

            self.send_command(TaskManagerCommand::ListProcesses(sender))
                .await?;

            Ok(receiver
                .await
                .expect("Response should always be sent if the send above was successful"))
        }

        #[remote_method]
        pub async fn reload(&self, reload_type: ReloadType) -> Result<()> {
            self.send_command(TaskManagerCommand::Reload(reload_type))
                .await
        }

        #[remote_method]
        pub async fn start(&self, program_name: String) -> Result<()> {
            self.send_command(TaskManagerCommand::StartProgram { program_name })
                .await
        }

        #[remote_method]
        pub async fn restart(&self, program_name: String) -> Result<()> {
            self.send_command(TaskManagerCommand::RestartProgram { program_name })
                .await
        }

        #[remote_method]
        pub async fn stop(&self, program_name: String) -> Result<()> {
            self.send_command(TaskManagerCommand::StopProgram { program_name })
                .await
        }

        #[remote_method]
        pub async fn stop_all_processes(&self) -> Result<()> {
            self.send_command(TaskManagerCommand::StopAllProcesses)
                .await
        }

        #[remote_method]
        pub async fn shutdown(&self) -> Result<()> {
            self.send_command(TaskManagerCommand::Shutdown).await
        }

        async fn send_command(
            &self,
            command: TaskManagerCommand,
        ) -> Result<(), ServerCommandError> {
            self.tasks_manager_handle.lock().await.send(command).await
        }
    }

    pub struct Client<Stream> {}
}
