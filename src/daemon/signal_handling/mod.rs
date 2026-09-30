use crate::daemon::{
    config_state::ReloadArgs,
    tasks_manager::{Handle, TaskManagerCommand},
};
use signal::Signal;
use std::ffi::{CStr, CString, c_char};
use std::io::{self, PipeReader, PipeWriter};
use std::os::fd::AsRawFd;
use std::sync::atomic::{AtomicI32, Ordering};
use std::{ffi::c_int, mem::size_of};

/// Read end of the self-pipe used to hand signals received in the signal handler over to the
/// async signal loop. `-1` means the pipe hasn't been created yet.
static READ_FD: AtomicI32 = AtomicI32::new(-1);
/// Write end of the self-pipe, written to by the signal handler. `-1` means the pipe hasn't been
/// created yet.
static WRITE_FD: AtomicI32 = AtomicI32::new(-1);

unsafe extern "C" {
    /// This function is implemented inside interface.c file (this is C code linking with the Rust binary)
    ///
    /// # Return value
    /// - On Success -> `NULL`
    /// - On Failure -> Result of `strerror(errno)` that doesn't need to be freed
    fn declare_sighandlers() -> *const c_char;
}

#[derive(Debug, thiserror::Error)]
#[error("error binding signal handler: sigaction failed: {0:?}")]
pub struct SigActionError(CString);

struct SignalChannel(#[allow(unused)] PipeReader, #[allow(unused)] PipeWriter);

impl SignalChannel {
    /// Creates the self-pipe. MUST be called before the signal handlers are installed so that
    /// `on_signal` never has to allocate or take a lock.
    fn init() -> Self {
        assert_eq!( WRITE_FD.load(Ordering::SeqCst), -1, "programmatic error: SignalChannel is already initialized, SignalChannel::init() was called twice");

        let (reader, writer) = io::pipe().expect("Signal channel failed to create pipe");
        READ_FD.store(reader.as_raw_fd(), Ordering::SeqCst);
        WRITE_FD.store(writer.as_raw_fd(), Ordering::SeqCst);
        Self(reader, writer)
    }

    fn recv() -> Result<i32, io::Error> {
        let fd = READ_FD.load(Ordering::SeqCst);
        let mut buf = [0u8; size_of::<i32>()];
        let mut filled = 0;

        while filled < buf.len() {
            let read = unsafe {
                libc::unistd::read(fd, buf[filled..].as_mut_ptr().cast(), buf.len() - filled)
            };

            match read {
                -1 => {
                    let err = io::Error::last_os_error();
                    if err.kind() == io::ErrorKind::Interrupted {
                        continue;
                    }
                    return Err(err);
                }
                0 => return Err(io::ErrorKind::UnexpectedEof.into()),
                read => filled += read as usize,
            }
        }

        Ok(i32::from_ne_bytes(buf))
    }

    /// Async-signal-safe: only performs a single raw `write`.
    fn send(signal: i32) {
        let fd = WRITE_FD.load(Ordering::SeqCst);
        if fd == -1 {
            return;
        }

        let bytes = signal.to_ne_bytes();
        unsafe { libc::unistd::write(fd, bytes.as_ptr().cast(), bytes.len()) };
    }
}

#[allow(unused)]
#[unsafe(no_mangle)]
extern "C" fn on_signal(signum: c_int) {
    SignalChannel::send(signum);
}


/// Creates the self-pipe and installs taskmaster's signal handlers. The pipe is created first so
/// that the handler installed below is only ever able to perform a raw `write`.
fn install_sighandlers() -> Result<SignalChannel, SigActionError> {
    let channel = SignalChannel::init();

    unsafe {
        // declare sighandler for SIGHUP and SIGINT
        let errno = declare_sighandlers();
        if !errno.is_null() {
            return Err(SigActionError(CStr::from_ptr(errno).to_owned()));
        }
    }

    Ok(channel)
}

pub async fn handle_signal(handle: &Handle) -> Result<(), SigActionError> {
    let channel = install_sighandlers()?;

    while let Ok(signum) = tokio::task::spawn_blocking(SignalChannel::recv)
        .await
        .expect("task panicked")
    {
        if react_to_signal(signum, handle).await {
            break;
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
        assert!(
            install_sighandlers().is_ok(),
            "installing signal handlers must succeed"
        );

        let handle = tokio::spawn(run_loop_and_assert_result());

        let pid = std::process::id();
        unsafe {
            kill(pid as i32, SIGNAL);
        }
        let _ = handle.await;
    }
}
