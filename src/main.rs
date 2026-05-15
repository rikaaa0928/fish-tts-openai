use axum::{
    body::Body,
    extract::{Request, State},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use reqwest::Client;
use serde::Serialize;
use serde_json::Value;
use std::{env, net::SocketAddr};
use tokio::net::TcpListener;
use tracing::{error, info};

#[derive(Clone)]
struct AppState {
    fish_tts_url: String,
    fish_tts_key: String,
    proxy_api_key: Option<String>,
    client: Client,
}

#[derive(Serialize, Debug)]
struct FishTtsPayload {
    text: String,
    reference_id: String,
    format: String,
    chunk_length: u64,
    temperature: f64,
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
        .route("/v1/audio/speech", post(create_speech))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("Starting Fish TTS to OpenAI Proxy on port {}...", port);

    let listener = TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn create_speech(
    State(state): State<AppState>,
    req: Request<Body>,
) -> Result<Response, StatusCode> {
    if let Some(proxy_key) = &state.proxy_api_key {
        let auth_header = req.headers().get(header::AUTHORIZATION);
        match auth_header {
            Some(header_val) => {
                let expected = format!("Bearer {}", proxy_key);
                if header_val.to_str().unwrap_or("") != expected {
                    return Err(StatusCode::UNAUTHORIZED);
                }
            }
            None => return Err(StatusCode::UNAUTHORIZED),
        }
    }

    // Read body manually instead of using Json extractor because we need to handle invalid json gracefully as 400
    let bytes = axum::body::to_bytes(req.into_body(), 2 * 1024 * 1024)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let body: Value = match serde_json::from_slice(&bytes) {
        Ok(b) => b,
        Err(_) => {
            return Ok((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"detail": "Invalid JSON body"})),
            )
                .into_response())
        }
    };

    let text = match body.get("input").and_then(|v| v.as_str()) {
        Some(t) if !t.is_empty() => t.to_string(),
        _ => {
            return Ok((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"detail": "Missing 'input' field"})),
            )
                .into_response())
        }
    };

    let voice = body
        .get("voice")
        .and_then(|v| v.as_str())
        .unwrap_or("shantianfang")
        .to_string();

    let response_format = body
        .get("response_format")
        .and_then(|v| v.as_str())
        .unwrap_or("mp3")
        .to_string();

    let chunk_length = body
        .get("chunk_length")
        .and_then(|v| v.as_u64())
        .unwrap_or(300);

    let temperature = body
        .get("temperature")
        .and_then(|v| v.as_f64())
        .unwrap_or(1.0);

    let fish_payload = FishTtsPayload {
        text,
        reference_id: voice,
        format: response_format.clone(),
        chunk_length,
        temperature,
    };

    let req_builder = state
        .client
        .post(&state.fish_tts_url)
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", state.fish_tts_key),
        )
        .header(header::ACCEPT, "application/json, audio/*")
        .json(&fish_payload);

    let fish_res = req_builder.send().await.map_err(|e| {
        error!("Failed to connect to Fish TTS: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let status = fish_res.status();
    if !status.is_success() {
        let err_text = fish_res.text().await.unwrap_or_default();
        return Ok((
            StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(serde_json::json!({"detail": format!("Fish TTS error: {}", err_text)})),
        )
            .into_response());
    }

    let media_type = match response_format.as_str() {
        "mp3" | "opus" | "aac" | "flac" => format!("audio/{}", response_format),
        "pcm" => "audio/pcm".to_string(),
        _ => "application/octet-stream".to_string(),
    };

    let stream = fish_res.bytes_stream();
    let body = Body::from_stream(stream);

    let mut response = Response::builder()
        .status(StatusCode::OK)
        .body(body)
        .unwrap();

    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(&media_type).unwrap(),
    );

    Ok(response)
}
