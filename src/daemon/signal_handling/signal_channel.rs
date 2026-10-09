use std::{
    io::{self, Read, Write},
    os::fd::{FromRawFd, IntoRawFd},
    sync::atomic::{AtomicI32, Ordering},
};

const FD_NOT_INITIALIZED: i32 = -1;
/// Read end of the self-pipe used to hand signals received in the signal handler over to the
/// async signal loop. `FD_NOT_INITIALIZED` means the pipe hasn't been created yet.
static READ_FD: AtomicI32 = AtomicI32::new(FD_NOT_INITIALIZED);
/// Write end of the self-pipe, written to by the signal handler. `FD_NOT_INITIALIZED` means the
/// pipe hasn't been created yet.
static WRITE_FD: AtomicI32 = AtomicI32::new(FD_NOT_INITIALIZED);

const ERRNO_NOT_USED: i32 = 0;
const ERRNO_BROKEN_PIPE: i32 = 32;
static WRITE_ERRNO: AtomicI32 = AtomicI32::new(ERRNO_NOT_USED);

#[derive(Debug, thiserror::Error)]
pub enum ChannelError {
    #[error("Error writing signal to signal channel")]
    WritingToPipe(#[source] io::Error),
    #[error("Failed to read signal from signal channel")]
    ReadingFromPipe(#[source] io::Error),
}

pub(super) struct SignalChannel;

impl SignalChannel {
    /// Creates the self-pipe. MUST be called before the signal handlers are installed so that
    /// `on_signal` never has to allocate or take a lock.
    pub(super) fn init() -> Self {
        assert_eq!(
            READ_FD.load(Ordering::SeqCst),
            FD_NOT_INITIALIZED,
            "programmatic error: SignalChannel is already initialized, SignalChannel::init() was called twice"
        );

        let (reader, writer) = io::pipe().expect("Signal channel failed to create pipe");
        READ_FD.store(reader.into_raw_fd(), Ordering::SeqCst);
        WRITE_FD.store(writer.into_raw_fd(), Ordering::SeqCst);

        Self {}
    }

    pub(super) unsafe fn recv() -> Result<i32, ChannelError> {
        let mut reader = unsafe { io::PipeReader::from_raw_fd(READ_FD.load(Ordering::SeqCst)) };
        let mut buffer = [0; 4];
        let result = reader.read_exact(&mut buffer);

        // converting reader back to RawFd to avoid Drop closing the pipe
        let _ = reader.into_raw_fd();

        if let Err(err) = result {
            match WRITE_ERRNO.load(Ordering::SeqCst) {
                ERRNO_NOT_USED => {}
                errno => {
                    return Err(ChannelError::WritingToPipe(io::Error::from_raw_os_error(
                        errno,
                    )));
                }
            }
            return Err(ChannelError::ReadingFromPipe(err));
        }
        Ok(i32::from_ne_bytes(buffer))
    }

    /// Async-signal-safe: only performs a single raw `write`.
    pub(super) unsafe fn send(signal: i32) {
        if WRITE_FD.load(Ordering::SeqCst) == FD_NOT_INITIALIZED {
            return;
        }

        let mut writer = unsafe { io::PipeWriter::from_raw_fd(WRITE_FD.load(Ordering::SeqCst)) };
        let result = writer.write_all(&signal.to_ne_bytes());
        if let Err(err) = result {
            WRITE_ERRNO.update(
                Ordering::SeqCst,
                Ordering::SeqCst,
                |current| match current {
                    ERRNO_NOT_USED if let Some(err) = err.raw_os_error() => err,
                    ERRNO_NOT_USED => ERRNO_BROKEN_PIPE,
                    current => current,
                },
            );

            WRITE_FD.store(FD_NOT_INITIALIZED, Ordering::SeqCst);
            return;
        }

        // converting writer back to RawFd to avoid Drop closing the pipe
        let _ = writer.into_raw_fd();
    }
}

impl Drop for SignalChannel {
    fn drop(&mut self) {
        match READ_FD.swap(FD_NOT_INITIALIZED, Ordering::SeqCst) {
            FD_NOT_INITIALIZED => {}
            read_fd => unsafe {
                // dropping a newly created reader, consequently closing the file descriptor
                io::PipeReader::from_raw_fd(read_fd);
            },
        }
        match WRITE_FD.swap(FD_NOT_INITIALIZED, Ordering::SeqCst) {
            FD_NOT_INITIALIZED => {}
            write_fd => unsafe {
                // dropping a newly created writer, consequently closing the file descriptor
                io::PipeWriter::from_raw_fd(write_fd);
            },
        }
    }
}
