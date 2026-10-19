use std::sync::PoisonError;

use crate::ARGS;

pub enum StartMode {
    Force,
    Acknowledge,
    Classic,
}

impl StartMode {
    pub fn from_args() -> Self {
        let mut vec = ARGS.lock().unwrap_or_else(PoisonError::into_inner);

        if vec.contains(&"--force".to_string()) || vec.contains(&"-f".to_string()) {
            Self::remove_force(&mut vec);
            Self::Force
        } else if vec.contains(&"-y".to_string()) {
            Self::remove_acknowledge(&mut vec);
            Self::Acknowledge
        } else {
            Self::Classic
        }
    }

    fn remove_force(vec: &mut Vec<String>) {
        vec.retain(|elem| !matches!(elem.as_str(), "--force" | "-f"));
    }

    fn remove_acknowledge(vec: &mut Vec<String>) {
        vec.retain(|elem| !matches!(elem.as_str(), "-y"));
    }
}
