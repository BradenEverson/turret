//! Server Service Struct Implementation

use futures::{executor, stream::SplitSink, StreamExt};
use futures_util::Future;
use http_body_util::Full;
use hyper::{
    body::{self, Bytes},
    service::Service,
    upgrade::Upgraded,
    Method, StatusCode,
};
use hyper::{Request, Response};
use std::{fs::File, io::Read, pin::Pin, sync::Arc, time::Duration};
use tokio::sync::RwLock;
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};

use super::ServerState;

/// A websocket write stream
pub type WebSocketWriteStream =
    SplitSink<WebSocketStream<hyper_util::rt::tokio::TokioIo<Upgraded>>, Message>;

/// A server service implementation responsible for updating the state
#[derive(Clone)]
pub struct ServerService {
    /// The internal state
    pub state: Arc<RwLock<ServerState>>,
}

impl ServerService {
    /// Creates a new server service from a thread safe ServerState
    pub fn new(state: Arc<RwLock<ServerState>>) -> Self {
        Self { state }
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

            let res = match *req.method() {
                Method::GET => {
                    let file = match req.uri().path() {
                        "/" => "index.html",
                        "/left" => {
                            let success = executor::block_on(async move {
                                let mut state = state.write().await;
                                state.step_pin.set_high()?;
                                tokio::time::sleep(Duration::from_millis(10)).await;
                                state.step_pin.set_low()?;

                                Ok::<(), i32>(())
                            });

                            match success {
                                Ok(_) => "wegood.html",
                                Err(_) => "wenotgood.html",
                            }
                        }
                        "/right" => {
                            let success = executor::block_on(async move {
                                let mut state = state.write().await;
                                state.direction_pin.set_high()?;
                                state.step_pin.set_high()?;
                                tokio::time::sleep(Duration::from_millis(10)).await;
                                state.step_pin.set_low()?;
                                state.direction_pin.set_low()?;

                                Ok::<(), i32>(())
                            });

                            match success {
                                Ok(_) => "wegood.html",
                                Err(_) => "wenotgood.html",
                            }
                        }
                        "/shoot" => {
                            println!("SHOOTING");

                            "wegood.html"
                        }
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
