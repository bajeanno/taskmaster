pub enum StartMode {
    Force,
    Classic,
}

impl StartMode {
    pub fn from_args() -> Self {
        match std::env::args()
            .skip(1)
            .any(|arg| matches!(arg.as_str(), "--force" | "-f"))
        {
            true => Self::Force,
            false => Self::Classic,
        }
    }
}
