use crate::config_state::ReloadArgs;
use crate::tasks_manager::{Handle, TaskManagerCommand};
use signal::Signal;
use std::ffi::{CStr, CString, c_char};
use std::sync::PoisonError;
use std::sync::mpsc::{self, Receiver, RecvError, SendError, Sender};
use std::{
    ffi::c_int,
    sync::{LazyLock, Mutex},
};
use thiserror::Error;

static SIGNAL_CHANNEL: LazyLock<SignalChannel> = LazyLock::new(SignalChannel::new);

#[allow(unused)]
unsafe extern "C" {
    /// This function is implemented inside interface.c file (this is C code linking with the Rust binary)
    ///
    /// # Return value
    /// - On Success -> `NULL`
    /// - On Failure -> Result of `strerror(errno)` that doesn't need to be freed
    fn declare_sighandlers() -> *const c_char;
}

#[derive(Debug, Error)]
#[error("error binding signal handler: sigaction failed: {0:?}")]
pub struct SigActionError(CString);

struct SignalChannel {
    sender: Sender<c_int>,
    receiver: Mutex<Receiver<c_int>>,
}

impl SignalChannel {
    fn new() -> Self {
        let (s, r) = mpsc::channel();
        Self {
            sender: s,
            receiver: Mutex::new(r),
        }
    }

    #[allow(unused)] // TODO: remove that
    fn recv() -> Result<i32, RecvError> {
        SIGNAL_CHANNEL
            .receiver
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .recv()
    }

    fn send(signal: i32) -> Result<(), SendError<i32>> {
        SIGNAL_CHANNEL.sender.send(signal)
    }
}

#[allow(unused)]
#[unsafe(no_mangle)]
pub extern "C" fn on_signal(signum: c_int) {
    SignalChannel::send(signum);
}

#[allow(unused)] // TODO: remove that
async fn handle_signal(handle: &Handle) -> Result<(), SigActionError> {
    unsafe {
        // declare sighandler for SIGHUP and SIGINT
        let errno = declare_sighandlers();
        if !errno.is_null() {
            return Err(SigActionError(CStr::from_ptr(errno).to_owned()));
        }
    }
    while let Ok(signum) = SignalChannel::recv() {
        if react_to_signal(signum, handle).await {
            break;
        }
    }
    Ok(())
}

/// Returns false if we should continue listening for signals
#[allow(unused)] // TODO: remove that
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
        eprintln!("Received signal {signum} and didn't recognized it, taskmaster will ignore it. \
            Please open an issue with the signal name or a pull \
            request with the solution to this issue");
    }
    false
}

#[cfg(test)]
mod test {
    use super::*;
    use libc::signal::kill;
    use std::time::Duration;
    use tokio::time::sleep;
    const SIGNAL: c_int = 1;

    async fn test_loop() {
        assert_eq!(SignalChannel::recv().unwrap(), SIGNAL);
    }

    async fn run_loop_and_assert_result() {
        tokio::select! {
            _ = test_loop() => {
            },
            _ = sleep(Duration::from_secs(10)) => {
                assert!(false);
            },
        }
    }

    #[tokio::test]
    async fn test_signal_handling() {
        unsafe {
            // declare sighandler for SIGHUP and SIGINT
            assert!(!declare_sighandlers().is_null());
        }

        let handle = tokio::spawn(run_loop_and_assert_result());

        let pid = std::process::id();
        unsafe {
            kill(pid as i32, SIGNAL);
        }
        let _ = handle.await;
    }
}
