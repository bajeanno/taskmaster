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

#[cfg(test)]
mod tests {
    use super::*;
    use std::assert_matches;

    #[test]
    fn force_from_args() {
        let mut vec = vec!["--force".to_string()];
        assert_matches!(StartMode::from_args(&mut vec), StartMode::Force);
        let mut vec = vec!["-f".to_string()];
        assert_matches!(StartMode::from_args(&mut vec), StartMode::Force);
        let mut vec = vec!["--force".to_string(), "-y".to_string()];
        assert_matches!(StartMode::from_args(&mut vec), StartMode::Force);
        let mut vec = vec!["-f".to_string(), "-y".to_string()];
        assert_matches!(StartMode::from_args(&mut vec), StartMode::Force);
    }

    #[test]
    fn acknowledge_from_args() {
        let mut vec = vec!["-y".to_string(), "-f".to_string()];
        assert_matches!(StartMode::from_args(&mut vec), StartMode::Acknowledge);
    }

    #[test]
    fn classic_from_args() {
        assert_matches!(
            StartMode::from_args(&mut vec!["".to_string()]),
            StartMode::Classic
        );
        assert_matches!(
            StartMode::from_args(&mut vec!["--y".to_string()]),
            StartMode::Classic
        );
        assert_matches!(
            StartMode::from_args(&mut vec!["force".to_string()]),
            StartMode::Classic
        );
        assert_matches!(
            StartMode::from_args(&mut vec!["acknowledge".to_string()]),
            StartMode::Classic
        );
    }
}
