use std::{
    io,
    sync::Arc,
    time::{Duration, Instant},
};

use actix_web::{
    middleware::Logger, web, web::Html, App, HttpRequest, HttpResponse, HttpServer, Responder,
};
use actix_ws::{AggregatedMessage, Session};
use bytestring::ByteString;
use futures_util::{stream::FuturesUnordered, StreamExt as _};
use tokio::sync::Mutex;
use webauthn_rs_core::proto;

#[derive(Clone)]
pub struct Chat {
    inner: Arc<Mutex<ChatInner>>,
}

pub struct ChatInner {
    sessions: Vec<Session>,
}
impl Chat {
    pub fn new() -> Self {
        Chat {
            inner: Arc::new(Mutex::new(ChatInner {
                sessions: Vec::new(),
            })),
        }
    }

    pub async fn insert(&self, session: Session) {
        self.inner.lock().await.sessions.push(session);
    }

    pub async fn send(&self, msg: impl Into<ByteString>) {
        let msg = msg.into();

        let mut inner = self.inner.lock().await;
        let mut unordered = FuturesUnordered::new();

        for mut session in inner.sessions.drain(..) {
            let msg = msg.clone();

            unordered.push(async move {
                let res = session.text(msg).await;
                res.map(|_| session)
                    .map_err(|_| println!("Dropping session"))
            });
        }   

        while let Some(res) = unordered.next().await {
            if let Ok(session) = res {
                inner.sessions.push(session);
            }
        }
    }
}
pub async fn ws(
    req: HttpRequest,
    body: web::Payload,
    chat: web::Data<Chat>,
) -> Result<HttpResponse, actix_web::Error> {
    println!("WebSocket connection request received: {:?}", req);
    let (response, mut session, stream) = actix_ws::handle(&req, body)?;
    println!("Websocket response generated");
    // increase the maximum allowed frame size to 128KiB and aggregate continuation frames
    let mut stream = stream.max_frame_size(128 * 1024).aggregate_continuations();

    chat.insert(session.clone()).await;
    tracing::info!("Inserted session");
    println!("Inserted session: ig");

    let alive = Arc::new(Mutex::new(Instant::now()));

    let mut session2 = session.clone();
    let alive2 = alive.clone();
    actix_web::rt::spawn(async move {
        let mut interval = actix_web::rt::time::interval(Duration::from_secs(5));

        loop {
            interval.tick().await;
            if session2.ping(b"you there ?").await.is_err() {
                break;
            }

            if Instant::now().duration_since(*alive2.lock().await) > Duration::from_secs(10) {
                let _ = session2.close(None).await;
                break;
            }
        }
    });

    actix_web::rt::spawn(async move {
        while let Some(Ok(msg)) = stream.recv().await {
            match msg {
                AggregatedMessage::Ping(bytes) => {
                    if session.pong(&bytes).await.is_err() {
                        return;
                    }
                }

                AggregatedMessage::Text(string) => {
                    println!("Relaying text, {string}");
                    chat.send(string).await;
                }

                AggregatedMessage::Close(reason) => {
                    let _ = session.close(reason.clone()).await;
                    println!("Got close, bailing {:?}",reason.clone());
                    return;
                }

                AggregatedMessage::Pong(_) => {
                    *alive.lock().await = Instant::now();
                }

                _ => (),
            };
        }
        let _ = session.close(None).await;
    });
    println!("Spawned");

    Ok(response)
}
