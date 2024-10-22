
use crate::{db_operations_repo::{poll_repo::{PollRepo, RepoError}, user_passkey_repo::UserRepo}, session, startup::UserData};
use actix_session::Session;
use actix_web::{web::{self, Data}, Error, HttpResponse};

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use uuid::Uuid;
use webauthn_rs::prelude::PasskeyAuthentication;

#[derive(Serialize,Deserialize)]
pub struct CreatePollRequest {
    title: String,
    creator: String,
    options: Vec<String>,
}

pub async fn create_poll(
    webauthn_users: Data<Mutex<UserData>>,
    req: web::Json<CreatePollRequest>,
) -> Result<HttpResponse,Error> {
    println!("entereed create_poll");
    let user_repo = UserRepo {client: &webauthn_users.lock().await.client };
    let user_id = user_repo
    .find_unique_id_by_username( &req.creator) 
    .await
    .map_err(|_| {actix_web::error::ErrorUnauthorized("Invalid user")})?;
    let id = match user_id {
        Some(id) => id,       
        None => {
            return Err(actix_web::error::ErrorUnauthorized("User not authenticated"));
        }
    };

    let repo = PollRepo { client: &webauthn_users.lock().await.client };
    println!("starting inserting poll");
    match repo.insert_poll(&req.title, id, &req.options).await {
        Ok(poll_id) => {
            println!("preparing the HttpResponse in json format");
            Ok(HttpResponse::Created().json(poll_id))
        },
        Err(_) => {
            Err(actix_web::error::ErrorInternalServerError(RepoError::DatabaseQueryError))
        }
    }
}