#[derive(Debug)]
pub enum StartMode {
    Force,
    Acknowledge,
    Classic,
}

impl StartMode {
    pub fn from_args(args: &mut Vec<String>) -> Self {
        if let Some(first) = args.first() {
            if first == "--force" || first == "-f" {
                args.remove(0);
                return Self::Force;
            }
            if first == "-y" {
                args.remove(0);
                return Self::Acknowledge;
            }
        }
        Self::Classic
    }

    pub fn is_force(&self) -> bool {
        matches!(self, Self::Force)
    }
}

    fn remove_force(vec: &mut Vec<String>) {
        vec.retain(|elem| !matches!(elem.as_str(), "--force" | "-f"));
    }

    fn remove_acknowledge(vec: &mut Vec<String>) {
        vec.retain(|elem| !matches!(elem.as_str(), "-y"));
    }
}
