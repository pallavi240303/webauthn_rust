use actix_cors::Cors;
use actix_session::SessionMiddleware;
use actix_web::{cookie::{Key, SameSite}, http, middleware, web, App, HttpServer};
use handlers::{handlers::{finish_authentication, register_finish, register_start, start_authentication}, polls_handlers::{close_poll, create_poll, get_all_polls_from_db, get_poll_details, manage_user_polls, reset_poll_votes, vote_on_poll}};
use log::info;
use session::MemorySession;
use startup::startup;
use web_socket_handlers::{start_connection::Chat, start_connection::ws};

mod db;
mod db_operations_repo;
mod startup;
mod handlers;
mod session;
mod web_socket_handlers;


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let (webauthn, webauthn_users) = startup().await;
   
    let chat = Chat::new();
    info!("Listening on: http://127.0.0.1:5500");
    let key = Key::generate();

    HttpServer::new(move || {
        
        let cors = Cors::default()
            .allowed_origin("http://localhost:3000") 
            .allowed_methods(vec!["GET", "POST", "OPTIONS"]) 
            .allowed_headers(vec![http::header::AUTHORIZATION, http::header::ACCEPT, http::header::CONTENT_TYPE])
            .allow_any_header() 
            .supports_credentials();


        App::new()
        .wrap(middleware::Logger::default())
        .wrap(
            SessionMiddleware::builder(MemorySession, key.clone())
            
                .cookie_name("webauthnrs".to_string())
                .cookie_http_only(true)
                .cookie_same_site(SameSite::Lax)
                .cookie_secure(false)
                .build(),
        )
        
        .wrap(cors)
            .app_data(webauthn.clone())
            .app_data(webauthn_users.clone()) 
            .app_data(web::Data::new(chat.clone()))
            .route("/register/start/{username}", web::post().to(register_start))
            .route("/register/finish", web::post().to(register_finish))
            .route("/login/start/{username}", web::post().to(start_authentication))
            .route("/login/finish",web::post().to(finish_authentication))
            .route("/poll/new", web::post().to(create_poll))
            .route("/polls",web::post().to(get_all_polls_from_db))
            .route("/polls/{poll_id}/vote",web::post().to(vote_on_poll))
            .route("/polls/{poll_id}",web::get().to(get_poll_details))
            .route("/ws", web::get().to(ws))
            .route("/polls/manage", web::post().to(manage_user_polls))
            .route("/polls/{poll_id}/close" , web::post().to(close_poll))
            .route("/polls/{poll_id}/reset" , web::post().to(reset_poll_votes))
        
    })
    .bind("127.0.0.1:5500")?
    .run()
    .await
}
