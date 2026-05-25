use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};

use crate::AppState;

pub async fn check_auth(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if let Some(proxy_key) = &state.proxy_api_key {
        let expected = format!("Bearer {}", proxy_key);
        match req.headers().get(header::AUTHORIZATION) {
            Some(val) if val.to_str().unwrap_or("") == expected => {}
            _ => return Err(StatusCode::UNAUTHORIZED),
        }
    }
    Ok(next.run(req).await)
}
