// TODO remove this file

use crate::session::Session;

pub struct PlaceHolder<T> {
    return_value: T,
}

#[derive(Debug)]
pub struct PlaceHolderError;

impl std::fmt::Display for PlaceHolderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PlaceHolderError")
    }
}

impl std::error::Error for PlaceHolderError {}

impl<T> PlaceHolder<T> {
    fn __new(return_value: T) -> Self {
        Self { return_value }
    }

    pub async fn call(self, _conn: &Session) -> Result<T, PlaceHolderError> {
        Ok(self.return_value)
    }
}

// #[rpc_genie::rpc]
pub fn list_tasks() -> PlaceHolder<Vec<String>> {
    PlaceHolder::__new(vec!["nginx".to_string(), "transcendence".to_string()])
}

// #[rpc_genie::rpc]
pub fn start(_task: String) -> PlaceHolder<Result<(), PlaceHolderError>> {
    PlaceHolder::__new(Ok(()))
}

// #[rpc_genie::rpc]
pub fn stop(_task: String) -> PlaceHolder<Result<(), PlaceHolderError>> {
    PlaceHolder::__new(Ok(()))
}

// #[rpc_genie::rpc]
pub fn restart(_task: String) -> PlaceHolder<Result<(), PlaceHolderError>> {
    PlaceHolder::__new(Ok(()))
}

// #[rpc_genie::rpc]
pub fn reload() -> PlaceHolder<Result<(), PlaceHolderError>> {
    PlaceHolder::__new(Ok(()))
}

// #[rpc_genie::rpc]
pub fn shutdown() -> PlaceHolder<Result<(), PlaceHolderError>> {
    PlaceHolder::__new(Ok(()))
}
