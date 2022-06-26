use std::collections::{BTreeMap};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Message {
    ClientMessage {
        message: String
    },
    Hello {
        username: String,
        channel: usize,
    },
    History {
        messages: BTreeMap<String, String>,
    },
    Quit,
    UserQuit {
        user: String,
    },
    SwitchChannel {
        new_channel: usize,
    },
    UsernameTaken,
    Ok,
    ChatFull,
    BroadcastMessage {
        message: String,
        user: String,
    },
    UserJoined {
        user: String,
    },
}
