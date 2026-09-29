use crate::config_state::ReloadArgs;
use crate::tasks_manager::{Handle, TaskManagerCommand};
use signal::Signal;
use std::ffi::{CStr, CString, c_char};
use std::io::{self, Read, Write, pipe};
use std::mem::MaybeUninit;
use std::sync::{Mutex, PoisonError};
use std::{ffi::c_int, sync::LazyLock};
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

pub struct SignalChannel {
    reader: Mutex<io::PipeReader>,
    writer: Mutex<io::PipeWriter>,
}

impl SignalChannel {
    fn new() -> Self {
        let (reader, writer) = pipe().expect("Signal channel failed to create pipe");
        Self {
            reader: Mutex::new(reader),
            writer: Mutex::new(writer),
        }
    }

    #[allow(unused)] // TODO: remove that
    pub fn recv() -> Result<i32, io::Error> {
        let mut buf: [MaybeUninit<u8>; 4] = unsafe { MaybeUninit::uninit().assume_init() };

        SIGNAL_CHANNEL
            .reader
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .read_exact(unsafe {
                std::slice::from_raw_parts_mut(buf.as_mut_ptr() as *mut u8, 4)
            })?;

        let buf: [u8; 4] = unsafe { std::mem::transmute(buf) };

        Ok(i32::from_ne_bytes(buf))
    }

    fn send(signal: i32) -> Result<(), io::Error> {
        SIGNAL_CHANNEL
            .writer
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .write_all(&signal.to_ne_bytes())
    }
}

#[allow(unused)]
#[unsafe(no_mangle)]
pub extern "C" fn on_signal(signum: c_int) {
    SignalChannel::send(signum);
}

#[allow(unused)] // TODO: remove that
pub async fn handle_signal(handle: &Handle) -> Result<(), SigActionError> {
    unsafe {
        // declare sighandler for SIGHUP and SIGINT
        let errno = declare_sighandlers();
        if !errno.is_null() {
            return Err(SigActionError(CStr::from_ptr(errno).to_owned()));
        }
    }

    while let Ok(signum) = tokio::task::spawn_blocking(SignalChannel::recv)
        .await
        .expect("task panicked")
    {
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
    const SIGNAL: c_int = 2;

    async fn test_loop() {
        assert_eq!(SignalChannel::recv().unwrap(), SIGNAL);
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
        unsafe {
            // declare sighandler for SIGHUP and SIGINT
            assert!(declare_sighandlers().is_null());
        }

        let handle = tokio::spawn(run_loop_and_assert_result());

        let pid = std::process::id();
        unsafe {
            kill(pid as i32, SIGNAL);
        }
        let _ = handle.await;
    }
}
