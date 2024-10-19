use tokio_postgres::{Client,  Error};
use postgres_native_tls::MakeTlsConnector;
use native_tls::TlsConnector;
use std::env;
use dotenv::dotenv;

pub async fn connect_db() -> Result<Client, Error> {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let connector = TlsConnector::builder().build().unwrap();
    let connector = MakeTlsConnector::new(connector);
    let (client, connection) = tokio_postgres::connect(&database_url, connector).await?;
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Connection error: {}", e);
        }
    });

    Ok(client)
}


// pub(crate) async fn get_user_by_username(client: &Client, username: &str) -> Result<Option<(i32, Uuid)>, tokio_postgres::Error> {
//     let row = client.query_opt("SELECT id, unique_id FROM users WHERE username = $1", &[&username]).await?;
//     Ok(row.map(|row| (row.get(0), row.get(1))))
// }

// // Function to create a new user
// pub(crate) async fn create_user(client: &Client, username: &str) -> Result<(i32, Uuid), tokio_postgres::Error> {
//     let unique_id = Uuid::new_v4();
//     let row = client.query_one(
//         "INSERT INTO users (unique_id, username) VALUES ($1, $2) RETURNING id",
//         &[&unique_id, &username],
//     ).await?;
//     Ok((row.get(0), unique_id))
// }

// // Function to get credentials by user ID
// pub(crate) async fn get_credentials_by_user_id(client: &Client, user_id: i32) -> Result<Vec<Credential>, tokio_postgres::Error> {
//     let rows = client.query(
//         "SELECT credential_id, public_key, counter FROM credentials WHERE user_id = $1",
//         &[&user_id],
//     ).await?;
    
//     let credentials = rows.into_iter().map(|row| {
//         Credential {
//             cred_id: row.get(0),
//             public_key: row.get(1),
//             counter: row.get(2),
//             id: todo!(),
//             credential_id: todo!(),
//         }
//     }).collect();
    
//     Ok(credentials)
// }

// // Function to insert a new credential
// pub(crate) async fn insert_credential(
//     client: &Client,
//     user_id: i32,
//     credential_id: &[u8],
//     public_key: &[u8],
//     counter: i32,
// ) -> Result<(), tokio_postgres::Error> {
//     client.execute(
//         "INSERT INTO credentials (user_id, credential_id, public_key, counter) VALUES ($1, $2, $3, $4)",
//         &[&user_id, &credential_id, &public_key, &counter],
//     ).await?;
//     Ok(())
// }

// pub(crate) async fn update_credential_counter(
//     client: &Client,
//     credential_id: &[u8],
//     new_counter: i32,
// ) -> Result<(), tokio_postgres::Error> {
//     client.execute(
//         "UPDATE credentials SET counter = $1 WHERE credential_id = $2",
//         &[&new_counter, &credential_id],
//     ).await?;
//     Ok(())
// }
