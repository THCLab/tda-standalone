use serde::{Deserialize, Serialize};
use std::{fmt, num::ParseIntError, str::FromStr};

#[derive(Serialize, Deserialize)]
pub struct Token {
    pub uuid: String,
}

impl PartialEq for Token {
    fn eq(&self, other: &Self) -> bool {
        self.uuid == other.uuid
    }
}

#[derive(Serialize, Deserialize)]
pub struct Event {
    pub uuid: String,
    msg: Message,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Message {
    payload: String,
}

impl FromStr for Message {
    type Err = ParseIntError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Message {
            payload: s.to_string(),
        })
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.payload)
    }
}
