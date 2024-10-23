
use std::collections::HashMap;

use actix_web::{HttpResponse, ResponseError};
use serde::{Deserialize, Serialize};
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

#[derive(Serialize, Deserialize)]
pub struct Poll {
    id: i32,               
    title: String,
    creator_id: Uuid,    
    options: Vec<PollOptions>,
}

#[derive(Serialize, Deserialize)]
pub struct PollDetails {
    pub id: i32,               
    pub title: String,
    pub creator_id: Uuid,
    pub closed : bool,    
    pub options: Vec<PollOptions>,
    pub created_at:  String
}

#[derive(Serialize, Deserialize,Debug)]
pub struct PollOptions {
    pub option_text : String,
    pub votes : i32
}


pub(crate) struct PollRepo<'a> {
    pub(crate) client: &'a Client,
}

impl<'a> PollRepo<'a> {
    pub async fn insert_poll(&self , title : &str , creator_id: Uuid, options : &Vec<String>) -> Result<i32 , RepoError> {
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
        let poll_id = row.ok_or(RepoError::DatabaseQueryError)?.get::<_, i32>(0);
        println!("poll id passed : {:?} ",poll_id);
        self.insert_poll_options(poll_id,  options).await.unwrap();
        println!("yeah going to insert into poll options noow");
        Ok(poll_id)
    }

    async fn insert_poll_options(&self , poll_id: i32 ,options: &Vec<String> ) -> Result<(),RepoError> {
        println!("inside poll_options function");
        println!("options are  : {:?}",options );
        println!("------");
        let mut query = String::from("INSERT INTO poll_options (poll_id, option_text) VALUES ");
    
    let mut params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = vec![];
    for (i, option) in options.iter().enumerate() {
        query.push_str(&format!("($1, ${}),", i + 2));
        params.push(option);
    }
    query.pop();
    params.insert(0, &poll_id);

    self.client
        .execute(&query, &params)
        .await
        .map_err(|e| {
            println!("error : {:?}", e);
            RepoError::DatabaseQueryError
        })?;
        println!("inserted!!!");
        Ok(())
    }

    pub async fn get_all_polls(&self) -> Result<Vec<Poll>, RepoError> {
        let query = r#"SELECT p.id, p.title, p.creator_id, 
                   po.option_text, 
                   po.votes 
            FROM polls p
            LEFT JOIN poll_options po ON p.id = po.poll_id
            GROUP BY p.id, p.title, p.creator_id, po.id"#; 

        let rows = self.client.query(query, &[]).await.map_err(|_| RepoError::DatabaseQueryError)?;
    
        let mut polls_map: HashMap<i32, Poll> = HashMap::new();
        for row in rows {
            let poll_id: i32 = row.get(0);
            let title: String = row.get(1);
            let creator_id: Uuid = row.get(2);
            let option_text: String = row.get(3);
            let vote_count: i32 = row.get(4);

            let poll = polls_map.entry(poll_id).or_insert(Poll {
                id: poll_id,
                title: title.clone(),
                creator_id: creator_id.clone(),
                options: Vec::new(),
            });

            poll.options.push(PollOptions {
                option_text,
                votes: vote_count,
            });
        }
        let polls: Vec<Poll> = polls_map.into_iter().map(|(_, poll)| poll).collect();
        Ok(polls)
    }

    pub async fn get_option_id_by_text_and_poll_id(&self , option_text: String , poll_id: i32) ->Result<Option<i32>, RepoError> {
        println!("inside get option by text and poll id");
        let query = "SELECT id from poll_options where option_text= $1 and poll_id = $2";
        let rows = self.client.query(query, &[&option_text, &poll_id]).await?;
        Ok(rows.iter().map(|row| row.get(0)).next())
    }

    pub async fn increment_vote_count(&self , option_id: i32) -> Result<(), RepoError>{
        println!("inside increment vote function");
        let query = "UPDATE poll_options SET votes = votes + 1 WHERE id = $1";
        self.client.execute(query, &[&option_id]).await?;
        Ok(())
    }

    pub async fn insert_vote_details(&self, user_id: Uuid, poll_id: i32, option_id: i32,voted_at:String) -> Result<(), RepoError> {
        println!("inside insert vote details function");
        let query = "INSERT INTO votes (user_id, poll_id, option_id, voted_at) VALUES ($1, $2, $3, $4)";
        self.client.execute(query, &[&user_id, &poll_id, &option_id , &voted_at]).await?;
        Ok(())
    }

    pub async fn get_poll_by_id(&self , poll_id: i32) -> Result<Option<PollDetails> ,RepoError>{
        println!("entered get poll by id function");
        let query = "select * from polls where id = $1";
        let row = self.client.query_opt(query, &[&poll_id]).await.expect("database query failed for get poll by id fn");
        print!("row : {:?}",row);
        match row { 
            Some(row) => Ok(Some(PollDetails {
                id : row.get("id"),
                title: row.get("title"),
                creator_id: row.get("creator_id"),
                created_at: row.get("created_at"),
                options: Vec::new(),
                closed: row.get("closed")
            })),
            None => Ok(None),
        }
    }

    pub async fn get_poll_options_with_votes(&self, poll_id: i32) -> Result<Vec<PollOptions>, RepoError> {
        let query = "SELECT option_text, votes FROM poll_options WHERE poll_id = $1";
        let rows = self.client.query(query, &[&poll_id]).await?;

        let options = rows.iter()
            .map(|row| PollOptions {
                option_text: row.get("option_text"),
                votes: row.get("votes"),
            })
            .collect();
            println!("options:  {:?}",options);
        Ok(options)
    }
}

