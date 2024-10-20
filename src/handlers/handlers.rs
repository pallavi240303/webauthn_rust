
use actix_session::{Session, SessionGetError, SessionInsertError};
use actix_web::{web::{Data, Json, Path}, HttpResponse };
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
    DeserialisationError,
    #[error("Username is not available")]
    UsernameUnavailable
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

        let user_exists = users_guard
            .client
            .query_one("SELECT unique_id FROM users WHERE username = $1", &[&username.to_string()])
            .await
            .is_ok();

        if user_exists {
            return Err(Error::UsernameUnavailable);
        }

        let unique_id_str = users_guard
            .client
            .query_one("SELECT unique_id FROM users WHERE username = $1", &[&username.to_string()])
            .await
            .map(|row| row.get(0))
            .unwrap_or_else(|_| Uuid::new_v4());
        unique_id_str
    };

    session.remove("reg_state");

    let (ccr , reg_state) = webauthn.start_passkey_registration(user_unique_id, &username, &username,None)
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
    session: Session,
    webauthn: Data<Webauthn>,
    webauthn_users: Data<Mutex<UserData>>,
) -> WebResult<HttpResponse> {
    let (username, user_unique_id, reg_state): (String, Uuid, PasskeyRegistration) = match session.get("reg_state")? {
        Some((username, user_unique_id, reg_state)) => (username, user_unique_id, reg_state),
        None => return Err(Error::CorruptSession),
    };

    let session_data: Option<(String, Uuid, PasskeyRegistration)> = session.get("reg_state")?;
    println!("Session data: {:?}", session_data);
    if session_data.is_none() {
        println!("No session data found for 'reg_state'");
    }

    println!("Attempting to finish registration with req: {:?}, reg_state: {:?}", req, reg_state);
    let sk = webauthn
        .finish_passkey_registration(&req, &reg_state)
        .map_err(|e| {
            println!("Error during passkey registration: {:?}", e);
            Error::BadRequest(e)
        })?;

    let sk_json = serde_json::to_value(&sk).unwrap();
    let users_guard = webauthn_users.lock().await;

    let user_exists_result = users_guard.client.query_opt(
        "SELECT id FROM users WHERE username = $1",
        &[&username],
    ).await;

    let user_exists = match user_exists_result {
        Ok(Some(_)) => true,
        Ok(None) => false,
        Err(e) => {
            println!("Error querying user: {:?}", e);
            return Err(Error::DatabaseQueryError);
        }
    };

    // If the user does not exist, insert the username into the users table
    if !user_exists {
        let unique_id = Uuid::new_v4();
        println!("unique id : {:?}", unique_id);
        println!("username : {:?}", username);
        println!("passkey data : {:?}", sk_json);

        let _ = users_guard.client.execute(
            "INSERT INTO users (unique_id, username) VALUES ($1, $2)",
            &[&unique_id, &username],
        ).await.map_err(|e| {
            println!("Database insert error: {:?}", e);
        });

        let _ = users_guard.client.execute(
            "INSERT INTO passkeys_data (user_id, passkey_data) VALUES ($1, $2)",
            &[&unique_id, &sk_json],
        ).await;
    } else {
        let _ = users_guard.client.execute(
            "INSERT INTO passkeys_data (user_id, passkey_data) VALUES ($1, $2)",
            &[&user_unique_id, &sk_json],
        ).await;
    }

    session.remove("reg_state");
    println!("response sent: {:?}", HttpResponse::Ok().finish());
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
            "SELECT unique_id FROM users WHERE username = $1",
            &[&username.to_string()],
        ).await.map_err(|e| {
            println!("Database query error: fetching the user details  {:?}", e);
            Error::DatabaseQueryError
        })?;

        match user_id_row {
            Some(row) => row.get::<_,Uuid>(0),
            None => return Err(Error::UserNotFound),
        }
    };

    let allow_credentials = {
        let users_guard = webauthn_users.lock().await;

        let rows = users_guard.client.query(
            "SELECT passkey_data FROM passkeys_data WHERE user_id = $1",
            &[&user_unique_id],
        ).await.map_err(|e| {
            println!("Database query error: fetching the passkey data  {:?}", e);
            Error::DatabaseQueryError
        })?;

        if let Some(row) = rows.get(0) {
            let pk_json: serde_json::Value = row.get(0);
            match serde_json::from_value(pk_json) {
                Ok(pk) => pk,
                Err(e) => {
                    println!("Passkey couldn't be deserialized - {:?}", e);
                    return Err(Error::DeserialisationError);
                }
            }
        } else {
            return Err(Error::UserHasNoCredentials);
        }
    };

    let (rcr, auth_state) = webauthn
        .start_passkey_authentication(&[allow_credentials])
        .map_err(|e| {
            println!("challenge_authenticate -> {:?}", e);
            Error::Unknown(e)
        })?;

    session.insert("auth_state", (user_unique_id, &auth_state))?;
    println!("Session auth state: {:?}", auth_state.clone());
        println!("started authentication");
    Ok(Json(rcr))
}




pub(crate) async fn finish_authentication(
    auth: Json<PublicKeyCredential>,
    session: Session,
    webauthn_users: Data<Mutex<UserData>>,
    webauthn: Data<Webauthn>,
) -> WebResult<HttpResponse> {
    println!("startedt finish authentication");
    let (user_unique_id , auth_state) : (Uuid, PasskeyAuthentication)= session.get("auth_state")?.ok_or(Error::CorruptSession)?;
    println!("Received auth data: {:?}", auth);

    

    let auth_result = webauthn
        .finish_passkey_authentication(&auth, &auth_state)
        .map_err(|e| {
            info!("challenge_register -> {:?}", e);
            Error::BadRequest(e)
        })?;
        println!("auth result  : {:?}",auth_result);

    let  users_guard = webauthn_users.lock().await;
    let rows = users_guard.client.query(
        "SELECT passkey_data FROM passkeys_data WHERE user_id = $1",
        &[&user_unique_id],
    ).await.map_err(|e| {
        println!("Database query error: {:?}", e);
        Error::DatabaseQueryError
    })?;

    if rows.is_empty() {
        return Err(Error::UserHasNoCredentials); 
    }

    for row in rows {
        let pk_json: serde_json::Value = row.get(0);

        let mut stored_passkey: Passkey = serde_json::from_value(pk_json.clone()).map_err(|e| {
            println!("Failed to deserialize passkey: {:?}", e);
            Error::DeserialisationError
        })?;

        stored_passkey.update_credential(&auth_result);

        let updated_passkey_json = serde_json::to_value(&stored_passkey).map_err(|e| {
            println!("Failed to serialize updated passkey: {:?}", e);
            Error::DeserialisationError
        })?;

        users_guard.client.execute(
            "UPDATE passkeys_data SET passkey_data = $1 WHERE user_id = $2 AND passkey_data = $3",
            &[&updated_passkey_json, &user_unique_id, &pk_json],
        ).await.map_err(|e| {
            println!("Database update error: {:?}", e);
            Error::DatabaseQueryError
        })?;
    }
    session.remove("auth_state");
    println!("Authentication Successful for user:");
    Ok(HttpResponse::Ok().finish())
}