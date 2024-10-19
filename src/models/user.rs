use uuid::Uuid;
use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: i32,
    pub unique_id: Uuid,
    pub username: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Credential {
    pub id: i32,
    pub user_id: i32,
    pub credential_id: Vec<u8>,
    pub public_key: Vec<u8>,
    pub counter: i32,
}

impl User {
    pub fn new(id: i32, unique_id: Uuid, username: String) -> Self {
        Self {
            id,
            unique_id,
            username,
        }
    }
}

impl Credential {
    pub fn new(id: i32, user_id: i32, credential_id: Vec<u8>, public_key: Vec<u8>, counter: i32) -> Self {
        Self {
            id,
            user_id,
            credential_id,
            public_key,
            counter,
        }
    }

}

