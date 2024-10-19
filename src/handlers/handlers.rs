
use std::collections::HashMap;

use actix_session::{Session, SessionGetError, SessionInsertError};
use actix_web::{web::{Data, Json, Path}, HttpResponse };
use base64urlsafedata::HumanBinaryData;
use log::{debug, error, info};
use tokio::sync::Mutex;
use webauthn_rs::prelude::*;
use actix_web::http::StatusCode;
use webauthn_rs::prelude::WebauthnError;
use crate::startup::UserData;
use thiserror::Error;
type WebResult<T> = Result<T, Error>;

#[derive(Debug, Error)]
pub(crate) enum Error {
    #[error("Unknown webauthn error")]
    Unknown(WebauthnError),
    #[error("Corrupt get session error")]
    SessionGet(#[from] SessionGetError),
    #[error("Corrupt insert session error")]
    SessionInsert(#[from] SessionInsertError),
    #[error("Corrupt session error")]
    CorruptSession,
    #[error("Bad request")]
    BadRequest(#[from] WebauthnError),
    #[error("User not found")]
    UserNotFound,
    #[error("User has no credentials")]
    UserHasNoCredentials,
    #[error("Database query error ")]
    DatabaseQueryError,
    #[error("Deserialization error ")]
    DeserialisationError
}

impl actix_web::ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}


pub(crate) async fn register_start(
    username: Path<String>,
    session:Session,
    webauthn_users: Data<Mutex<UserData>>,
    webauthn: Data<Webauthn>,
) -> WebResult<Json<CreationChallengeResponse>> {
    info!("Start Register");
    let user_unique_id = {
        let users_guard = webauthn_users.lock().await;
        let unique_id_str = users_guard
            .client.query_one("SELECT unique_id FROM users WHERE username = $1", &[&username.to_string()])
            .await
            .map(|row| row.get::<_, String>(0))
            .unwrap_or_else(|_| Uuid::new_v4().to_string());
        Uuid::parse_str(&unique_id_str).unwrap_or_else(|_| Uuid::new_v4())
    };

    session.remove("reg_state");

    let exclude_credentials = {
        let users_guard = webauthn_users.lock().await;
        let exclude_credentials_query = users_guard.client.query_one(
            "SELECT credential_id FROM credentials WHERE user_id = $1",
            &[&user_unique_id.to_string()],
        ).await;

        match exclude_credentials_query {
            Ok(row) => {
                let credential_id: Vec<u8> = row.get(0);
                Some(vec![HumanBinaryData::from(credential_id)]) 
            }
            Err(_) => None,
        }
    };

    let (ccr , reg_state) = webauthn.start_passkey_registration(user_unique_id, &username, &username, exclude_credentials)
    .map_err(|e| {
        debug!("Challenge_register -> {:?}",e);
        Error::Unknown(e)
    })?;

    if let Err(err) = session.insert("reg_state", (username.as_str(), user_unique_id, reg_state)) {
        error!("Failed to save reg_state to session storage!");
        return Err(Error::SessionInsert(err));
    };


    info!("Registeration initiation successful");
    Ok(Json(ccr))
}

pub(crate) async fn register_finish(
    req: Json<RegisterPublicKeyCredential>,
    session:Session,
    webauthn: Data<Webauthn>,
    webauthn_users: Data<Mutex<UserData>>,
) -> WebResult<HttpResponse> {
    let cookies: Vec<_> = session
        .get::<HashMap<String, String>>("reg_state") // Change this based on your session data structure
        .unwrap_or_else(|_| {
            error!("Failed to retrieve session data");
            None
        })
        .unwrap_or_default()
        .into_iter()
        .collect();
    
    println!("Received cookies: {:?}", cookies);

    let (username, user_unique_id, reg_state): (String,String, PasskeyRegistration)  = match session.get("reg_state")? {
        Some((username, user_unique_id, reg_state)) => (username, user_unique_id, reg_state),
        None => return {
            debug!("Session state 'reg_state' not found.");
            Err(Error::CorruptSession)
        },
    };

    session.remove("reg_state");

    let sk = webauthn
        .finish_passkey_registration(&req, &reg_state)
        .map_err(|e| {
            info!("Attestation response: {:?}", req);
            info!("Registration state: {:?}", reg_state);
            info!("Challenge_register -> {:?}", e);
            Error::BadRequest(e)
        })?;

        let sk_bytes = bincode::serialize(&sk).unwrap();
        let users_guard = webauthn_users.lock().await;

        let user_exists = users_guard.client.query_opt(
            "SELECT id FROM users WHERE username = $1",
            &[&username],
        ).await.is_ok();
        // If the user does not exist, insert the username into the users table
        if !user_exists {
            let unique_id = Uuid::new_v4(); // Generate a new UUID for the user
            let _ = users_guard.client.execute(
                "INSERT INTO users (unique_id, username) VALUES ($1, $2)",
                &[&unique_id.to_string(), &username],
            ).await.map_err(|e| {
                error!("Database insert error: {:?}", e);
            });
           
        }
       
        let _ = users_guard.client.execute(
            "INSERT INTO passkeys (user_id, passkey_data) VALUES ($1, $2)",
            &[&user_unique_id,&sk_bytes],
        ).await;


    Ok(HttpResponse::Ok().finish())
}

pub(crate) async fn start_authentication(
    username: Path<String>,
    session: Session,
    webauthn_users: Data<Mutex<UserData>>,
    webauthn: Data<Webauthn>,
) -> WebResult<Json<RequestChallengeResponse>> {
    info!("Start Authentication");

    session.remove("auth_state");

    let user_unique_id = {
        let users_guard = webauthn_users.lock().await;

        let user_id_row = users_guard.client.query_opt(
            "SELECT id FROM users WHERE username = $1",
            &[&username.to_string()],
        ).await.map_err(|e| {
            error!("Database query error: {:?}", e);
            Error::DatabaseQueryError 
        })?;

        match user_id_row {
            Some(row) => row.get::<_, i32>(0), 
            None => return Err(Error::UserNotFound), 
        }
    };

    let allow_credentials  = {
        let users_guard = webauthn_users.lock().await;

        let rows = users_guard.client.query(
            "SELECT passkey_data FROM passkeys WHERE user_id = $1",
            &[&user_unique_id],
        ).await.map_err(|e| {
            error!("Database query error: {:?}", e);
            Error::DatabaseQueryError 
        })?;
        if let Some(row) = rows.get(0){
            let pk_bytes : &[u8] = row.get(0);
            
            match bincode::deserialize::<Passkey>(pk_bytes) {
             Ok(pk) => pk,
             Err(e)  => {
                error!("Passkey couldnt be Deserialized - {:?}",e);
                return Err(Error::DeserialisationError); 
             }
            }
        }else {
            return Err(Error::UserHasNoCredentials); 
        }
    };

    let (rcr, auth_state) = webauthn
        .start_passkey_authentication(&[allow_credentials])
        .map_err(|e| {
            info!("challenge_authenticate -> {:?}", e);
            Error::Unknown(e)
        })?;

    session.insert("auth_state", (user_unique_id, auth_state))?;

    Ok(Json(rcr))
}

pub(crate) async fn finish_authentication(
    auth: Json<PublicKeyCredential>,
    session: Session,
    webauthn_users: Data<Mutex<UserData>>,
    webauthn: Data<Webauthn>,
) -> WebResult<HttpResponse> {
    let (user_unique_id , auth_state) : (String, PasskeyAuthentication)= session.get("auth_state")?.ok_or(Error::CorruptSession)?;

    session.remove("auth_state");

    let auth_result = webauthn
        .finish_passkey_authentication(&auth, &auth_state)
        .map_err(|e| {
            info!("challenge_register -> {:?}", e);
            Error::BadRequest(e)
        })?;

    let  users_guard = webauthn_users.lock().await;
    let rows = users_guard.client.query(
        "SELECT passkey_data FROM passkeys WHERE user_id = $1",
        &[&user_unique_id],
    ).await.map_err(|e| {
        error!("Database query error: {:?}", e);
        Error::DatabaseQueryError
    })?;

    if rows.is_empty() {
        return Err(Error::UserHasNoCredentials); 
    }

    for row in rows {
        let pk_bytes: &[u8] = row.get(0);

        let mut stored_passkey: Passkey = bincode::deserialize(pk_bytes).map_err(|e| {
            error!("Failed to deserialize passkey: {:?}", e);
            Error::DeserialisationError
        })?;

    
        stored_passkey.update_credential(&auth_result);

        let updated_passkey_bytes = bincode::serialize(&stored_passkey).map_err(|e| {
            error!("Failed to serialize updated passkey: {:?}", e);
            Error::DeserialisationError
        })?;

        users_guard.client.execute(
            "UPDATE passkeys SET passkey_data = $1 WHERE user_id = $2 AND passkey_data = $3",
            &[&updated_passkey_bytes, &user_unique_id, &pk_bytes],
        ).await.map_err(|e| {
            error!("Database update error: {:?}", e);
            Error::DatabaseQueryError
        })?;
    }

    info!("Authentication Successful for user:");
    Ok(HttpResponse::Ok().finish())
}