use std::sync::Arc;
use futures::{SinkExt, StreamExt};
use http_body_util::Full;
use hyper::{Request, Response, StatusCode};
use hyper::body::{Bytes, Incoming};
use hyper::upgrade::Upgraded;
use hyper_util::rt::TokioIo;
use jsonwebtoken::{decode, DecodingKey, Validation};
use tokio::sync::Mutex;
use tokio_tungstenite::WebSocketStream;
use tungstenite::Message;
use url::Url;
use crate::config::websocket::AppConfig;
use crate::entities::socket::{ChannelAuth, ConnectionManager};

type BoxError = Box<dyn std::error::Error + Send + Sync>;
async fn handle_websocket_connection(
    websocket: WebSocketStream<TokioIo<Upgraded>>,
    manager: Arc<Mutex<ConnectionManager>>,
    token_data: ChannelAuth,
) {
    println!("second point");
    let (mut write, mut read) = websocket.split();

    let mut manager = manager.lock().await;
    let mut receiver = manager.subscribe(
        token_data.channel_name.clone(),
        token_data.user_id,
    ).await;

    println!("third point");

    write.send(Message::Text("Connected to private channel".into())).await
        .expect("Failed to send connection confirmation");

    loop {
        match receiver.recv().await {
            Ok(msg) => {
                let message = serde_json::to_string(&msg)
                    .expect("Failed to serialize message");
                if write.send(Message::Text(message)).await.is_err() {
                    break;
                }
            }
            Err(_) => break,
        }
    }
}
pub async fn handle_request(
    req: Request<Incoming>,
    manager: Arc<Mutex<ConnectionManager>>,
    config: Arc<AppConfig>,
) -> Result<Response<Full<Bytes>>, BoxError> {
    println!("Received request");

    if !hyper_tungstenite::is_upgrade_request(&req) {
        println!("Not a websocket upgrade request");
        return Ok(Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Full::new(Bytes::from("Not a websocket upgrade request")))?);
    }
    println!("Is websocket upgrade request");

    // Проверяем токен до апгрейда
    let uri = req.uri().to_string();
    let url = Url::parse(&format!("http://localhost{}", uri))?;
    let token = url.query_pairs()
        .find(|(key, _)| key == "token")
        .map(|(_, value)| value.into_owned())
        .ok_or_else(|| BoxError::from("Missing token"))?;

    let token_data = match decode::<ChannelAuth>(
        &token,
        &DecodingKey::from_secret(&config.jwt_secret.as_bytes()),
        &Validation::default(),
    ) {
        Ok(data) => data.claims,
        Err(e) => {
            println!("Token decode error: {:?}", e);
            return Ok(Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .body(Full::new(Bytes::from("Invalid token")))?);
        }
    };

    println!("Attempting websocket upgrade");

    // Выполняем апгрейд и сразу же возвращаем ответ
    let (response, websocket) = hyper_tungstenite::upgrade(req, Default::default())?;

    let manager_clone = manager.clone();

    tokio::spawn(async move {
        println!("Waiting for websocket...");
        match websocket.await {
            Ok(ws) => {
                println!("Websocket connected successfully");
                handle_websocket_connection(ws, manager_clone, token_data).await;
            },
            Err(e) => {
                println!("Websocket connection error: {:?}", e);
            }
        }
    });

    // Важно! Возвращаем оригинальный response от upgrade
    Ok(response.map(|_| Full::new(Bytes::new())))
}