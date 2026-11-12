use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Default, PartialEq, Clone)]
pub enum AutoRestart {
    #[serde(rename = "true")]
    True,
    #[default]
    #[serde(rename = "false")]
    False,
    #[serde(rename = "unexpected")]
    OnFailure,
}
