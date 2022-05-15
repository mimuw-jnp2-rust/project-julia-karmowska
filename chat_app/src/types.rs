use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub id: Uuid,
    pub user: User,
    pub body: String,
    pub group: Uuid,
    pub time: DateTime<Utc>,
}

impl Message {
    pub fn new(id: Uuid, user: User, msg: &str, group: Uuid, time: DateTime<Utc>) -> Self {
        Message {
            id,
            user,
            body: String::from(msg),
            group,
            time,
        }
    }
}