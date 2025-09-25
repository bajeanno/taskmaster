mod call_error;
pub use call_error::CallError;

mod cast_error;
pub use cast_error::CastError;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Call(CallError),
    Cast(CastError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Call(error) => write!(f, "{error}"),
            Self::Cast(error) => write!(f, "{error}"),
        }
    }
}

impl core::error::Error for Error {}

impl From<CastError> for Error {
    fn from(error: CastError) -> Self {
        Self::Cast(error)
    }
}

impl From<CallError> for Error {
    fn from(error: CallError) -> Self {
        Self::Call(error)
    }
}
