mod auth;
mod references;
mod speech;

use axum::{middleware, routing::{get, post}, Router};
use reqwest::Client;
use std::{env, net::SocketAddr};
use tokio::net::TcpListener;
use tracing::info;

#[derive(Clone)]
pub struct AppState {
    pub fish_tts_url: String,
    pub fish_tts_key: String,
    pub proxy_api_key: Option<String>,
    pub client: Client,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let _ = dotenvy::dotenv();

    let fish_tts_url = env::var("FISH_TTS_URL")
        .unwrap_or_else(|_| "http://192.168.0.69:8080/v1/tts".to_string());
    let fish_tts_key = env::var("FISH_TTS_KEY").unwrap_or_else(|_| "key".to_string());
    let proxy_api_key = env::var("PROXY_API_KEY").ok();
    let port: u16 = env::var("PORT")
        .unwrap_or_else(|_| "8000".to_string())
        .parse()
        .expect("PORT must be a number");

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .expect("Failed to build HTTP client");

    let state = AppState {
        fish_tts_url,
        fish_tts_key,
        proxy_api_key,
        client,
    };

    let app = Router::new()
        .route("/v1/tts", post(speech::proxy_tts))
        .route("/v1/audio/speech", post(speech::create_speech))
        .route("/v1/references/add", post(references::add_reference))
        .route("/v1/references/list", get(references::list_references))
        .route("/v1/references/delete", post(references::delete_reference))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::check_auth,
        ))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("Starting Fish TTS to OpenAI Proxy on port {}...", port);

    let listener = TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
