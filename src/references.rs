use axum::{
    body::Body,
    extract::{Request, State},
    http::{header, StatusCode},
    response::Response,
};
use tracing::error;

use crate::AppState;

fn fish_base_url(tts_url: &str) -> &str {
    tts_url.trim_end_matches("/v1/tts").trim_end_matches('/')
}

pub async fn add_reference(
    State(state): State<AppState>,
    req: Request<Body>,
) -> Result<Response, StatusCode> {
    let content_type = req
        .headers()
        .get(header::CONTENT_TYPE)
        .cloned()
        .ok_or(StatusCode::BAD_REQUEST)?;

    let body_bytes = axum::body::to_bytes(req.into_body(), 50 * 1024 * 1024)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let base = fish_base_url(&state.fish_tts_url);
    let fish_res = state
        .client
        .post(format!("{}/v1/references/add", base))
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", state.fish_tts_key),
        )
        .header(header::CONTENT_TYPE, content_type)
        .header(header::ACCEPT, "application/json")
        .body(body_bytes)
        .send()
        .await
        .map_err(|e| {
            error!("Failed to connect to Fish TTS: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    proxy_response(fish_res).await
}

pub async fn list_references(
    State(state): State<AppState>,
) -> Result<Response, StatusCode> {
    let base = fish_base_url(&state.fish_tts_url);
    let fish_res = state
        .client
        .get(format!("{}/v1/references/list", base))
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", state.fish_tts_key),
        )
        .header(header::ACCEPT, "application/json")
        .send()
        .await
        .map_err(|e| {
            error!("Failed to connect to Fish TTS: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    proxy_response(fish_res).await
}

pub async fn delete_reference(
    State(state): State<AppState>,
    req: Request<Body>,
) -> Result<Response, StatusCode> {
    let body_bytes = axum::body::to_bytes(req.into_body(), 2 * 1024 * 1024)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let base = fish_base_url(&state.fish_tts_url);
    let fish_res = state
        .client
        .post(format!("{}/v1/references/delete", base))
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", state.fish_tts_key),
        )
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::ACCEPT, "application/json")
        .body(body_bytes)
        .send()
        .await
        .map_err(|e| {
            error!("Failed to connect to Fish TTS: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    proxy_response(fish_res).await
}

async fn proxy_response(resp: reqwest::Response) -> Result<Response, StatusCode> {
    let status = StatusCode::from_u16(resp.status().as_u16())
        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

    let mut builder = Response::builder().status(status);

    if let Some(ct) = resp.headers().get(header::CONTENT_TYPE) {
        builder = builder.header(header::CONTENT_TYPE, ct.clone());
    }

    let stream = resp.bytes_stream();
    builder
        .body(Body::from_stream(stream))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
