use std::fmt::Display;

#[derive(Debug, PartialEq)]
pub struct Command {
    pub exec: String,
    pub args: Vec<String>,
}

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}",
            self.exec,
            if !self.args.is_empty() {
                format!(" {:?}", self.args)
            } else {
                String::new()
            }
        )
    }
}
