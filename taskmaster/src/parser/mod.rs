pub mod program;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error(
        "Error opening taskmaster config file: {0}\n\
     Consider making a reload request after creating one"
    )]
    OpeningFile(#[from] std::io::Error),
    #[error(
        "Error parsing taskmaster config file: {0}\n\
     Consider making a reload request after fixing the issue"
    )]
    InvalidConfig(#[from] serde_yaml::Error),
}
