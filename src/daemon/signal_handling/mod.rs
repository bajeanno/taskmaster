mod signal_channel;

use crate::daemon::{
    config_state::ReloadArgs,
    tasks_manager::{Handle, TaskManagerCommand},
};
use signal::Signal;
use signal_channel::SignalChannel;
use std::ffi::{CStr, CString, c_char, c_int};

unsafe extern "C" {
    /// This function is implemented inside interface.c file (this is C code linking with the Rust binary)
    ///
    /// # Return value
    /// - On Success -> `NULL`
    /// - On Failure -> Result of `strerror(errno)` that doesn't need to be freed
    fn declare_sighandlers() -> *const c_char;
}

#[derive(Debug, thiserror::Error)]
pub enum SignalError {
    #[error("error binding signal handler: sigaction failed: {0:?}")]
    SigAction(CString),

    #[error("Signal channel was broken")]
    Channel(#[from] signal_channel::ChannelError),
}

#[allow(unused)]
#[unsafe(no_mangle)]
extern "C" fn on_signal(signum: c_int) {
    unsafe {
        SignalChannel::send(signum);
    };
}

/// Creates the self-pipe and installs taskmaster's signal handlers. The pipe is created first so
/// that the handler installed below is only ever able to perform a raw `write`.
fn install_sighandlers() -> Result<SignalChannel, SignalError> {
    let channel = SignalChannel::init();

    unsafe {
        // declare sighandler for SIGHUP and SIGINT
        let errno = declare_sighandlers();
        if !errno.is_null() {
            return Err(SignalError::SigAction(CStr::from_ptr(errno).to_owned()));
        }
    }

    Ok(channel)
}

pub async fn handle_signal(handle: &Handle) -> Result<(), SignalError> {
    let channel = install_sighandlers()?;

    loop {
        match tokio::task::spawn_blocking(|| unsafe { SignalChannel::recv() })
            .await
            .expect("task panicked")
        {
            Ok(signum) => {
                if react_to_signal(signum, handle).await {
                    break; // signum was 1: SIGINT meaning "exit"
                }
            }
            Err(err) => return Err(err.into()),
        }
    }
    drop(channel);
    Ok(())
}

/// Returns false if we should continue listening for signals
async fn react_to_signal(signum: c_int, handle: &Handle) -> bool {
    if let Ok(signal) = Signal::from_c_int(signum) {
        match signal {
            Signal::SIGHUP => {
                let _ = handle
                    .send(TaskManagerCommand::Reload(ReloadArgs::UseCurrent))
                    .await;
            }

            Signal::SIGINT => {
                let _ = handle.send(TaskManagerCommand::Exit).await;
                return true;
            }

            _ => {
                eprintln!(
                    "Received signal {signum}, this signal is not meant to be handled by \
                    taskmaster, please open an issue with the signal name or a pull \
                    request with the solution to this issue"
                );
            }
        }
    } else {
        eprintln!(
            "Received signal {signum} and didn't recognized it, taskmaster will ignore it. \
            Please open an issue with the signal name or a pull \
            request with the solution to this issue"
        );
    }
    false
}

#[cfg(test)]
mod test {
    use super::*;
    use libc::signal::kill;
    use std::time::Duration;
    use tokio::time::sleep;
    const TEST_SIGNAL: c_int = 2;

    async fn test_loop() {
        assert_eq!(unsafe { SignalChannel::recv().unwrap() }, TEST_SIGNAL);
    }

    async fn run_loop_and_assert_result() {
        tokio::select! {
            _ = test_loop() => {
            },
            _ = sleep(Duration::from_secs(1)) => {
                assert!(false);
            },
        }
    }

    #[tokio::test]
    async fn test_signal_handling() {
        assert!(
            install_sighandlers().is_ok(),
            "installing signal handlers must succeed"
        );

        let handle = tokio::spawn(run_loop_and_assert_result());

        let pid = std::process::id();
        unsafe {
            kill(pid as i32, TEST_SIGNAL);
        }
        let _ = handle.await;
    }
}
