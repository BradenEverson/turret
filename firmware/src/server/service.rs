//! Server Service Struct Implementation

use futures::{stream::SplitSink, StreamExt};
use futures_util::Future;
use http_body_util::Full;
use hyper::{
    body::{self, Bytes},
    service::Service,
    upgrade::Upgraded,
    Method, StatusCode,
};
use hyper::{Request, Response};
use std::{fs::File, io::Read, pin::Pin, sync::Arc};
use tokio::sync::{
    mpsc::{UnboundedReceiver, UnboundedSender},
    RwLock,
};
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};

use crate::turret::Action;

use super::ServerState;

/// A websocket write stream
pub type WebSocketWriteStream =
    SplitSink<WebSocketStream<hyper_util::rt::tokio::TokioIo<Upgraded>>, Message>;

/// A server service implementation responsible for updating the state
#[derive(Clone)]
pub struct ServerService {
    /// The internal state
    pub state: Arc<RwLock<ServerState>>,
    /// Action queue sender
    pub action_sender: UnboundedSender<Action>,
}

impl ServerService {
    /// Creates a new server service from a thread safe ServerState
    pub fn new(state: Arc<RwLock<ServerState>>) -> (Self, UnboundedReceiver<Action>) {
        let (writer, reader) = tokio::sync::mpsc::unbounded_channel();
        (
            Self {
                state,
                action_sender: writer,
            },
            reader,
        )
    }
}

impl Service<Request<body::Incoming>> for ServerService {
    type Response = Response<Full<Bytes>>;
    type Error = hyper::http::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, mut req: Request<body::Incoming>) -> Self::Future {
        let state = self.state.clone();
        if hyper_tungstenite::is_upgrade_request(&req) {
            let (response, websocket) =
                hyper_tungstenite::upgrade(&mut req, None).expect("Error upgrading to WebSocket");

            tokio::spawn(async move {
                let ws = websocket.await.expect("Error awaiting websocket handshake");
                let (writer, _) = ws.split();
                {
                    state.write().await.assign_websocket(writer);
                }
            });

            Box::pin(async { Ok(response) })
        } else {
            let response = Response::builder().status(StatusCode::OK);

            let send_channel = self.action_sender.clone();

            let res = match *req.method() {
                Method::GET => {
                    let file = match req.uri().path() {
                        "/" => "index.html",
                        "/left" => match send_channel.send(Action::Left) {
                            Ok(_) => "wegood.html",
                            Err(_) => "wenotgood.html",
                        },
                        "/right" => match send_channel.send(Action::Right) {
                            Ok(_) => "wegood.html",
                            Err(_) => "wenotgood.html",
                        },
                        "/shoot" => match send_channel.send(Action::Shoot) {
                            Ok(_) => "wegood.html",
                            Err(_) => "wenotgood.html",
                        },
                        _ => "404.html",
                    };

                    let file = format!("frontend/{}", file);
                    let mut page = File::open(file).expect("Failed to open file");
                    let mut buf = vec![];

                    page.read_to_end(&mut buf)
                        .expect("Failed to read to buffer");

                    response.body(Full::new(Bytes::copy_from_slice(&buf)))
                }
                _ => response.body(Full::new(Bytes::copy_from_slice(&[]))),
            };

            Box::pin(async { res })
        }
    }
}
