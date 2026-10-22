pub enum StartMode {
    Force,
    Acknowledge,
    Classic,
}

impl StartMode {
    pub fn from_args(args: &mut Vec<String>) -> Self {
        if args.contains(&"--force".to_string()) || args.contains(&"-f".to_string()) {
            Self::remove_force(args);
            Self::Force
        } else if args.contains(&"-y".to_string()) {
            Self::remove_acknowledge(args);
            Self::Acknowledge
        } else {
            Self::Classic
        }
    }

    pub fn is_force(&self) -> bool {
        matches!(self, Self::Force)
    }

    fn remove_force(vec: &mut Vec<String>) {
        vec.retain(|elem| !matches!(elem.as_str(), "--force" | "-f"));
    }

    fn remove_acknowledge(vec: &mut Vec<String>) {
        vec.retain(|elem| !matches!(elem.as_str(), "-y"));
    }
}
