
use actix_web::web::Data;
use tokio::sync::Mutex;
use tokio_postgres::Client;
use webauthn_rs::prelude::*;

use crate::db::db::connect_db;


pub(crate) struct UserData {
    pub(crate) client: Client,
}


pub(crate) async fn startup() -> (Data<Webauthn>, Data<Mutex<UserData>>){
    let rp_id = "localhost";
    let rp_origin = Url::parse("http://localhost:3000").expect("Invalid URL");
    let builder = WebauthnBuilder::new(rp_id, &rp_origin).expect("Invalid configuration");
    let builder = builder.rp_name("Actix-web webauthn-rs");
    let webauthn = Data::new(builder.build().expect("Invalid configuration"));
    let client = connect_db().await.unwrap();
    let webauthn_users = Data::new(Mutex::new(UserData {
        client,
    }));
    (webauthn, webauthn_users)
}