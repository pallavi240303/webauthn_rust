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
            println!("Connection error: {}", e);
        }
    });
    Ok(client)
}
