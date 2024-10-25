
use std::time::Duration;

use crate::{db_operations_repo::{poll_repo::{PollDetails, PollOptions, PollRepo, RepoError}, user_passkey_repo::UserRepo},startup::UserData, web_socket_handlers::start_connection::Chat};
use actix_session::Session;
use actix_web::{web::{self, Data}, Error, HttpResponse};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::{sync::{broadcast::{self, Sender}, Mutex}, time::timeout};
use uuid::Uuid;

#[derive(Serialize,Deserialize)]
pub struct CreatePollRequest {
    title: String,
    creator: String,
    options: Vec<String>,
}


#[derive(Deserialize)]
pub struct PollQueryParams {
    creator: Option<Uuid>, // Creator ID, optional
    live: Option<bool>,    // Whether the poll is live, optional
    closed: Option<bool>,  // Whether the poll is closed, optional
}

#[derive(Serialize, Deserialize)]
pub struct Poll {
    id: i32,               
    title: String,
    creator_id: Uuid,    
    options: Vec<String>,
    
}

#[derive(Deserialize,Debug)]
pub struct VoteRequest {
    username: String,
    option_text: String,
}

#[derive(Serialize, Deserialize , Clone , Debug)]
pub struct VoteUpdate {
    poll_id: i32,
    option_id: i32,
    votes: i32,
}

pub async fn create_poll(
    webauthn_users: Data<Mutex<UserData>>,
    req: web::Json<CreatePollRequest>,
) -> Result<HttpResponse,Error> {
    println!("entereed create_poll");
    let user_repo = UserRepo {client: &webauthn_users.lock().await.client };
    let user_id_result = timeout(Duration::from_secs(5), user_repo.find_unique_id_by_username(&req.creator))
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Timeout while fetching user"))?.unwrap();

    let id = match user_id_result {
        Some(id) => id,
        None => {
            return Err(actix_web::error::ErrorUnauthorized("User not authenticated"));
        }
    };
    print!("id:  {:?}",id);

    let repo = PollRepo { client: user_repo.client };
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

pub async fn get_all_polls_from_db(webauthn_users: Data<Mutex<UserData>> , query: web::Query<PollQueryParams>) -> Result<HttpResponse,Error> {
    let repo = PollRepo {client : &webauthn_users.lock().await.client };
    let PollQueryParams { creator, live, closed } = query.into_inner();
    match repo.get_polls_filtered(creator, live, closed).await {
        Ok(polls) =>{ 
            let response = serde_json::json!(polls);
            Ok(HttpResponse::Ok().json(response))
        },
        Err(e) => Err(actix_web::error::ErrorInternalServerError(format!("Error fetching polls: {:?}", e))),
    }
}

pub async fn manage_user_polls(webauthn_users: Data<Mutex<UserData>>,session:Session) -> Result<HttpResponse, Error>{
    let repo = PollRepo { client: &webauthn_users.lock().await.client };
    let user_id: Uuid = match session.get("user_unique_id").expect("Session get error") {
        Some(id) => id, 
        None => {
            return Err(actix_web::error::ErrorUnauthorized("User not authenticated"));
        } 
    };
    println!("user id from session:  {:?}",user_id);
    match repo.get_polls_by_creator(user_id).await {
        Ok(polls) => {
            let response_data = serde_json::json!({
                "user_id": user_id,
                "polls": polls
            });
            Ok(HttpResponse::Ok().json(response_data))
        },
        Err(e) => Err(actix_web::error::ErrorInternalServerError(format!("Error fetching user polls: {:?}", e))),
    }
}

pub async fn vote_on_poll(
    webauthn_users: Data<Mutex<UserData>>,
    poll_id: web::Path<i32>,
    req: web::Json<VoteRequest>,
    chat: web::Data<Chat>,
) -> Result<HttpResponse,Error> {


    let user_repo = UserRepo{client: &webauthn_users.lock().await.client};
    let user_id_result = user_repo.find_unique_id_by_username(&req.username).await;
    let user_id = match user_id_result {
        Ok(Some(id)) => id,
        Ok(None) => {
            println!("User not found");
            return Err(actix_web::error::ErrorUnauthorized("User not found"))
        },
        Err(_) => {
            println!("Error fetching user");
            return Err(actix_web::error::ErrorInternalServerError("Error fetching user"))
        }
    };

    let repo = PollRepo {client : &user_repo.client};

    let option_id_result = repo.get_option_id_by_text_and_poll_id(req.option_text.clone(), *poll_id).await;
    let option_id = match option_id_result {
        Ok(Some(id)) => id,
        Ok(None) => {
            println!("Option not found");
            return Err(actix_web::error::ErrorNotFound("Option not found"))
        },
        Err(_) => {
            println!("Error fetching option");
            return Err(actix_web::error::ErrorInternalServerError("Error fetching option"))
        },
    };

    if let Err(_) = repo.increment_vote_count(option_id).await {
        println!("Error incrementing vote_count");
        return Err(actix_web::error::ErrorInternalServerError("Error updating/incrementing vote count"));
    }

    let voted_at = Utc::now().to_string();
    if let Err(_) = repo.insert_vote_details(user_id, *poll_id, option_id,voted_at ).await {
        return Err(actix_web::error::ErrorInternalServerError("Error inserting vote record"));
    }

    let updated_vote_count = repo.get_vote_count(option_id).await.unwrap_or(0);
    let updated_message = format!("{{\"poll_id\": {}, \"option_text\": \"{}\", \"vote_count\": {}}}",poll_id, &req.option_text, updated_vote_count);
    println!("updated_message : {:?}",updated_message);
    chat.send(updated_message).await;
    println!("msg sent ig");
    Ok(HttpResponse::Ok().json("Vote recorded successfully"))
}

pub async fn get_poll_details(
    webauthn_users: Data<Mutex<UserData>>,
    poll_id: web::Path<i32>
) -> Result<HttpResponse,Error> {
    let repo = PollRepo { client: &webauthn_users.lock().await.client };
    let poll_details = match repo.get_poll_by_id(*poll_id).await {
        Ok(Some(poll)) => poll,
        Ok(None) => return Err(actix_web::error::ErrorNotFound("Poll not found")),
        Err(_) => return Err(actix_web::error::ErrorInternalServerError("Error fetching poll details")),
    };

    let poll_options = match repo.get_poll_options_with_votes(*poll_id).await {
        Ok(options) => options,
        Err(_) => return Err(actix_web::error::ErrorInternalServerError("Error fetching poll options")),
    };

    let response = PollDetails {
        id: poll_details.id,
        title: poll_details.title,
        creator_id: poll_details.creator_id,
        closed : poll_details.closed,
        created_at:poll_details.created_at,
        options: poll_options.into_iter()
            .map(|opt| PollOptions {
                option_text: opt.option_text,
                votes: opt.votes,
            })
            .collect(),
    };

    Ok(HttpResponse::Ok().json(response))
}

pub async fn close_poll(
    webauthn_users: Data<Mutex<UserData>>,
    session:Session,
    poll_id: web::Path<i32>,
    chat: web::Data<Chat>
) -> Result<HttpResponse ,Error> {
    let user_id: Uuid = match session.get("user_unique_id").expect("Session get error") {
        Some(id) => id, 
        None => {
            return Err(actix_web::error::ErrorUnauthorized("User not authenticated"));
        } 
    };


    let repo = PollRepo { client: &webauthn_users.lock().await.client };

    match repo.is_poll_creator(user_id, *poll_id).await {
        Ok(true) => {}, // User is the creator, proceed
        Ok(false) => return Ok(HttpResponse::Forbidden().body("You are not the creator of this poll")),
        Err(_) => return Ok(HttpResponse::InternalServerError().body("Error verifying poll creator")),
    }

    if repo.close_poll(*poll_id).await.is_err(){
        return Ok(HttpResponse::InternalServerError().body("Error closing the poll"));
    }

    let close_message = format!("{{\"poll_id\": {}, \"closed\": {}}}",poll_id, true);
    println!("updated_message : {:?}",close_message);

    chat.send(close_message).await;

    Ok(HttpResponse::Ok().body("Poll closed successfully"))
}

pub async fn reset_poll_votes(
    poll_id: web::Path<i32>,
    session:Session,
    webauthn_users: web::Data<Mutex<UserData>>,
) -> Result<HttpResponse, Error> {
    let repo = PollRepo { client: &webauthn_users.lock().await.client };
    let user_id: Uuid = match session.get("user_unique_id").expect("Session get error") {
        Some(id) => id, 
        None => {
            return Err(actix_web::error::ErrorUnauthorized("User not authenticated"));
        } 
    };
    match repo.is_poll_creator(user_id, *poll_id).await {
        Ok(true) => {
            // Reset votes
            match repo.reset_votes(*poll_id).await {
                Ok(_) => Ok(HttpResponse::Ok().json("Poll votes reset successfully")),
                Err(e) => Err(actix_web::error::ErrorInternalServerError(format!("Error resetting poll votes: {:?}", e))),
            }
        },
        Ok(false) => Err(actix_web::error::ErrorUnauthorized("You are not the creator of this poll")),
        Err(e) => Err(actix_web::error::ErrorInternalServerError(format!("Error: {:?}", e))),
    }
}