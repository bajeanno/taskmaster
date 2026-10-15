use std::fmt::Display;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SerializableError {
    error: String,
    source: Option<Box<Self>>,
}

impl SerializableError {
    pub fn new(error: impl core::error::Error) -> Self {
        Self {
            error: error.to_string(),
            source: error.source().map(|source| Box::new(Self::new(source))),
        }
    }
}

impl Display for SerializableError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.error)
    }
}

impl core::error::Error for SerializableError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_deref()
            .map(|source| source as &(dyn core::error::Error + 'static))
    }
}
