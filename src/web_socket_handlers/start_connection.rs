use crate::web_socket_handlers::ws::WsConn;
use crate::web_socket_handlers::lobby::Lobby;
use actix::Addr;
use actix_web::{web::Data, web::Path, web::Payload, Error, HttpResponse, HttpRequest};
use actix_web_actors::ws;

use super::messages::{GroupCreationRetrieve, NotifyPollId};


pub async fn start_connection(
    req: HttpRequest,
    stream: Payload,
    poll_id: Path<i32>,
    srv: Data<Addr<Lobby>>,
) -> Result<HttpResponse, Error> {
    println!("entered websocket start_connection function");
    let group_id = {
        let lobby = srv.get_ref().clone();
        match lobby.send(GroupCreationRetrieve {
            poll_id: poll_id.into_inner(),
        }).await
        {
            Ok(Ok(gid)) => gid,
            Ok(Err(e)) => {
                return Err(actix_web::error::ErrorInternalServerError(format!("Error:  {:?}" ,e)))
            }
            Err(e) => return Err(actix_web::error::ErrorInternalServerError(e)) 
        }
    };
    let ws = WsConn::new(group_id, srv.get_ref().clone());
    let resp = ws::start(ws, &req, stream)?;
    println!("websocket started: {:?}",resp);
    Ok(resp)
}

pub async fn notify_poll(
    poll_id: Path<i32>,
    srv: Data<Addr<Lobby>>
) -> Result<HttpResponse,Error> {
    srv.send(NotifyPollId {
        poll_id: poll_id.clone(),
    }).await.map_err(|e| {
        eprintln!("Error sending message to lobby : {:?}",e);
        actix_web::error::ErrorInternalServerError(e)
    })?;
    Ok(HttpResponse::Ok().body(format!("Notified lobby for poll id:  {}" , poll_id)))
}