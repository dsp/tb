//! HTTP route handlers.

pub mod accounts;
pub mod frontend;
pub mod transfers;

use crate::api::HealthResponse;
use crate::state::AppState;
use axum::extract::State;
use axum::Json;
use std::sync::Arc;

/// Health check endpoint.
pub async fn health(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    let tb_connected = {
        let client = state.client.lock().await;
        client.is_ready()
    };

    Json(HealthResponse {
        status: "ok".to_string(),
        tb_connected,
    })
}
