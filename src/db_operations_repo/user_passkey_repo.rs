use tokio_postgres::Client;
use serde_json::Value;
use uuid::Uuid;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RepoError {
    #[error("Database query error")]
    DatabaseQueryError,
}

pub(crate) struct UserRepo<'a> {
    pub(crate) client: &'a Client,
}

impl<'a> UserRepo<'a> {
    // Fetch unique ID for a username
    pub(crate) async fn find_unique_id_by_username(
        &self,
        username: &str
    ) -> Result<Option<Uuid>, RepoError> {
        self.client
            .query_opt("SELECT unique_id FROM users WHERE username = $1", &[&username])
            .await
            .map(|row| row.map(|r| r.get(0)))
            .map_err(|_| RepoError::DatabaseQueryError)
    }

    // Insert a new user
    pub(crate) async fn insert_user(
        &self,
        unique_id: &Uuid,
        username: &str
    ) -> Result<(), RepoError> {
        self.client
            .execute(
                "INSERT INTO users (unique_id, username) VALUES ($1, $2)",
                &[unique_id, &username],
            )
            .await
            .map_err(|_| RepoError::DatabaseQueryError)?;
        Ok(())
    }

    //Fetch passkeys for a user id 
    pub async fn find_passkeys_by_user_id(
        &self, 
        user_id: &Uuid
    ) -> Result<Option<serde_json::Value>, RepoError> {
        let row = self.client
            .query_opt("SELECT passkey_data FROM passkeys_data WHERE user_id = $1", &[user_id])
            .await.unwrap();
        
        Ok(row.map(|r| r.get(0)))
    }

    // Insert a passkey for a user
    pub(crate) async fn insert_passkey(
        &self,
        user_id: &Uuid,
        passkey_data: &Value
    ) -> Result<(), RepoError> {
        self.client
            .execute(
                "INSERT INTO passkeys_data (user_id, passkey_data) VALUES ($1, $2)",
                &[user_id, passkey_data],
            )
            .await
            .map_err(|_| RepoError::DatabaseQueryError)?;
        Ok(())
    }

    // Update an existing passkey
    pub(crate) async fn update_passkey(
        &self,
        user_id: &Uuid,
        old_data: &Value,
        new_data: &Value
    ) -> Result<(), RepoError> {
        self.client
            .execute(
                "UPDATE passkeys_data SET passkey_data = $1 WHERE user_id = $2 AND passkey_data = $3",
                &[new_data, user_id, old_data],
            )
            .await
            .map_err(|_| RepoError::DatabaseQueryError)?;
        Ok(())
    }
}
