
use std::{collections::HashMap, hash::Hash};

use actix_web::{HttpResponse, ResponseError};
use thiserror::Error;
use uuid::Uuid;
use tokio_postgres::Client;
use chrono::{NaiveDateTime, Utc};


#[derive(Debug, Error)]
pub enum RepoError {
    #[error("Database query error")]
    DatabaseQueryError,
    #[error("At least 2 unique options are required")]
    NotEnoughOptions,
    #[error("Duplicate options are not allowed")]
    DuplicateOptions,
    #[error("A database error occurred")]
    DatabaseError(#[from] tokio_postgres::Error),
}

impl ResponseError for RepoError {
    fn error_response(&self) -> HttpResponse {
        match self {
            RepoError::NotEnoughOptions => HttpResponse::BadRequest().body(self.to_string()),
            RepoError::DuplicateOptions => HttpResponse::BadRequest().body(self.to_string()),
            RepoError::DatabaseError(_) => HttpResponse::InternalServerError().body("Database error"),
            RepoError::DatabaseQueryError => HttpResponse::InternalServerError().body("Database Query Error")
        }
    }
}

pub(crate) struct PollRepo<'a> {
    pub(crate) client: &'a Client,
}

impl<'a> PollRepo<'a> {
    pub async fn insert_poll(&self , title : &str , creator_id: Uuid, options : &Vec<String>) -> Result<u32 , RepoError> {
        println!("inside the insert_poll function");
        let query  = "INSERT INTO polls (title, creator_id ,created_at) VALUES ($1,$2,$3) RETURNING id";
        let curr_time : NaiveDateTime = Utc::now().naive_utc();
        let row = self.client
        .query_opt(query, &[&title , &creator_id , &curr_time.to_string()])
        .await.map_err(|e| {
            println!("error : {:?}",e);
            RepoError::DatabaseQueryError 
        })?;
        println!("inserted successfully ig");
        let poll_id = row.ok_or(RepoError::DatabaseQueryError)?.get::<_, u32>(0);
        self.insert_poll_options(poll_id,  options).await.unwrap();
        println!("yeah going to insert into poll options noow");
        Ok(poll_id)
    }

    async fn insert_poll_options(&self , poll_id: u32 ,options: &Vec<String> ) -> Result<(),RepoError> {
        println!("inside poll_options function");
        let query  =  "INSERT INTO poll_options (poll_id,option_text) VALUES ($1,$2)";
        for option in options {
            self.client
                .query_one(query, &[&poll_id , option])
                .await
                .map_err(|e| {
                    println!("error : {:?}",e);
                    RepoError::DatabaseQueryError 
                })?;
        }
        println!("inserted!!!");
        Ok(())
    }

    

    
}



// pub(crate) async fn create_poll(
//     req: Json<CreatePollRequest>,
//     session: Session,
//     webauthn_users: Data<Mutex<UserData>>, 
// ) -> Result<HttpResponse, actix_web::Error> {

//     // Ensure the user is logged in and get the user ID from session
//     let creator_id: Uuid = session.get("user_unique_id")?.ok_or_else(|| {
//         HttpResponse::Unauthorized().body("User is not authenticated")
//     }).unwrap();

//     let CreatePollRequest { title, options } = req.into_inner();

//     // Validate the number of options (at least 2 required)
//     if options.len() < 2 {
//         return Err(RepoError::NotEnoughOptions.into());
//     }

//     // Validate no duplicate options
//     let unique_options: std::collections::HashSet<_> = options.iter().collect();
//     if unique_options.len() != options.len() {
//         return Err(RepoError::DuplicateOptions.into());
//     }

//     // Insert the poll into the "polls" table
//     let poll_insert_query = "
//         INSERT INTO polls (title, creator_id, created_at)
//         VALUES ($1, $2, $3)
//         RETURNING id
//     ";
//     let poll_id: i32 = db_client.query_one(
//         poll_insert_query,
//         &[&title, &creator_id, &Utc::now()]
//     ).await
//     .map_err(|_err| PollError::DatabaseError)?;

//     // Insert the options into the "poll_options" table
//     let option_insert_query = "
//         INSERT INTO poll_options (poll_id, option_text)
//         VALUES ($1, $2)
//     ";
//     for option in options {
//         db_client.execute(
//             option_insert_query,
//             &[&poll_id, &option],
//         ).await
//         .map_err(|_err| PollError::DatabaseError)?;
//     }

//     Ok(HttpResponse::Ok().json(CreatePollResponse {
//         poll_id,
//         message: "Poll created successfully".to_string(),
//     }))
// }
