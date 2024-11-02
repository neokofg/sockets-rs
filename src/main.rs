extern crate core;

use std::sync::Arc;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use crate::config::websocket::{AppConfig};
use crate::entities::socket::ConnectionManager;
use crate::services::socket_service::{handle_request};

mod entities;
mod services;
mod config;
type BoxError = Box<dyn std::error::Error + Send + Sync>;
#[derive(Serialize)]
struct Claims {
    user_id: i32,
    channel_name: String,
    exp: i32
}

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    let secret = "test";
    let config = Arc::new(AppConfig {
        jwt_secret: secret.to_string(),
    });
    let claims = Claims {
        user_id: 1,
        channel_name: "private-chat".to_string(),
        exp: 2000000000, // Some future timestamp
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.jwt_secret.as_bytes())
    ).unwrap();

    println!("Token: {}", token);
    let manager = Arc::new(Mutex::new(ConnectionManager::new()));

    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr).await?;
    println!("Listening on http://{}", addr);

    loop {
        let (stream, _) = listener.accept().await?;
        // TokioIo для асинхронной работы с TCP-стримом
        let io = TokioIo::new(stream);

        let config = config.clone();
        let manager = manager.clone();

        tokio::task::spawn(async move {
            if let Err(err) = hyper::server::conn::http1::Builder::new()
                .serve_connection(
                    io,
                    service_fn(move |req| handle_request(req, manager.clone(), config.clone())),
                )
                .await
            {
                eprintln!("Error serving connection: {:?}", err);
            }
        });
    }
}
