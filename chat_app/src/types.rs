use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use serde_json;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub name: String,
}
impl User {
    pub fn new(id: Uuid, name: &str) -> Self {
        User {
            id,
            name: String::from(name),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]

struct Text{
    pub time : DateTime<Utc>,
    pub user :String,
    pub text: String,
}


#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Message {
    ClientMessage {
        message:String
    },
    Hello {
        username: String,
        channel: usize,
    },
    History {
        messages: BTreeMap<String, String>,
    },
    Quit,
    SwitchChannel {
        new_channel: usize,
    },
    UsernameTaken,
    Ok,
    ChatFull,
    BroadcastMessage{
        message:String,
        user:String
    }
}
pub struct ServerMessage{
    user:User,
    text: String,
}