
use actix_session::{Session, SessionGetError, SessionInsertError};
use actix_web::{web::{Data, Json, Path}, HttpResponse };
use log::{debug, error, info};
use tokio::sync::Mutex;
use webauthn_rs::prelude::*;
use actix_web::http::StatusCode;
use webauthn_rs::prelude::WebauthnError;
use crate::{db_operations_repo::user_passkey_repo::UserRepo, startup::UserData};
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
    #[error("Serialization error ")]
    SerialisationError,
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

    let repo = UserRepo { client : &webauthn_users.lock().await.client};

    // Check if user exists
    let user_exists = repo.find_unique_id_by_username(&username).await.unwrap().is_some();
    if user_exists {
        return Err(Error::UsernameUnavailable);
    }
        
    // Generate new unique ID
    let user_unique_id = Uuid::new_v4();
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
    let (username, user_unique_id, reg_state): (String, Uuid, PasskeyRegistration) =
        session.get("reg_state")?.ok_or(Error::CorruptSession)?;

    let repo = UserRepo { client: &webauthn_users.lock().await.client };

    // Finish WebAuthn registration
    let sk = webauthn
        .finish_passkey_registration(&req, &reg_state)
        .map_err(|e| {
            println!("Error during passkey registration: {:?}", e);
            Error::BadRequest(e)
        })?;

    let sk_json = serde_json::to_value(&sk).unwrap();

    // Check if the user exists, insert the user and passkey if not
    if repo.find_unique_id_by_username(&username).await.unwrap().is_none() {
        let unique_id = Uuid::new_v4();
        repo.insert_user(&unique_id, &username).await.unwrap();
        repo.insert_passkey(&unique_id, &sk_json).await.unwrap();
    } else {
        repo.insert_passkey(&user_unique_id, &sk_json).await.unwrap();
    }

    session.remove("reg_state");
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

    let repo = UserRepo { client: &webauthn_users.lock().await.client };

    let user_unique_id = {
        let user_id = repo.find_unique_id_by_username(&username).await.map_err(|e| {
            println!("Database query error: fetching the user details  {:?}", e);
            Error::DatabaseQueryError
        })?;
        match user_id {
            Some(id) => id,
            None =>  return Err(Error::UserNotFound)
        }
    };


    let allow_credentials = {
        let rows = repo.find_passkeys_by_user_id(&user_unique_id).await.map_err(|e| {
            println!("Database query error: fetching the passkey data  {:?}", e);
            Error::DatabaseQueryError
        })?;

        let pk_json = rows.ok_or(Error::UserHasNoCredentials)?;
        serde_json::from_value(pk_json).map_err(|e| {
            println!("Passkey couldn't be deserialized - {:?}", e);
            Error::DeserialisationError
        })?
    };

    let (rcr, auth_state) = webauthn
        .start_passkey_authentication(&[allow_credentials])
        .map_err(|e| {
            println!("challenge_authenticate -> {:?}", e);
            Error::Unknown(e)
        })?;

    session.insert("auth_state", (user_unique_id, &auth_state))?;
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
    
    let auth_result = webauthn
        .finish_passkey_authentication(&auth, &auth_state)
        .map_err(|e| {
            info!("challenge_register -> {:?}", e);
            Error::BadRequest(e)
        })?;
        println!("auth result  : {:?}",auth_result);

    let repo = UserRepo { client: &webauthn_users.lock().await.client };

    let stored_passkey_json = repo.find_passkeys_by_user_id(&user_unique_id)
        .await.unwrap()
        .ok_or(Error::UserHasNoCredentials)?;

    let mut stored_passkey: Passkey = serde_json::from_value(stored_passkey_json.clone())
    .map_err(|e| {Error::DeserialisationError})?;

    stored_passkey.update_credential(&auth_result);

    let updated_passkey_json = serde_json::to_value(&stored_passkey)
        .map_err(|e| {Error::SerialisationError})?;

    repo.update_passkey(&user_unique_id, &stored_passkey_json , &updated_passkey_json).await.unwrap();
    session
        .insert("user_unique_id", user_unique_id)
        .map_err(|_| Error::CorruptSession)?;

    println!("Authentication Successful for user: {:?}", user_unique_id);
    Ok(HttpResponse::Ok().finish())
}