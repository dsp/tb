//! Transfer route handlers.

use crate::api::{ApiTransfer, TransfersResponse};
use crate::error::AppError;
use crate::html;
use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::response::{Html, IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use std::sync::Arc;
use tb_rs::{QueryFilter, QueryFilterFlags};

/// Check if request is from HTMX.
fn is_htmx_request(headers: &HeaderMap) -> bool {
    headers.contains_key("hx-request")
}

/// Query parameters for listing transfers.
#[derive(Debug, Deserialize)]
pub struct ListTransfersParams {
    /// Filter by ledger.
    pub ledger: Option<u32>,
    /// Filter by code.
    pub code: Option<u16>,
    /// Maximum number of results.
    #[serde(default = "default_limit")]
    pub limit: u32,
    /// Pagination: start after this timestamp.
    pub after_timestamp: Option<u64>,
    /// Return in reverse chronological order.
    #[serde(default)]
    pub reversed: bool,
}

fn default_limit() -> u32 {
    100
}

/// List transfers with optional filters.
pub async fn list_transfers(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(params): Query<ListTransfersParams>,
) -> Result<Response, AppError> {
    let mut flags = QueryFilterFlags::empty();
    if params.reversed {
        flags |= QueryFilterFlags::REVERSED;
    }

    let filter = QueryFilter {
        user_data_128: 0,
        user_data_64: 0,
        user_data_32: 0,
        ledger: params.ledger.unwrap_or(0),
        code: params.code.unwrap_or(0),
        timestamp_min: params.after_timestamp.map(|t| t + 1).unwrap_or(0),
        timestamp_max: 0,
        limit: params.limit,
        flags,
        reserved: [0; 6],
    };

    let transfers = {
        let client = state.client.lock().await;
        client.query_transfers(filter).await?
    };

    let next_timestamp = transfers.last().map(|t| t.timestamp);
    let api_transfers: Vec<ApiTransfer> = transfers.iter().map(ApiTransfer::from).collect();

    if is_htmx_request(&headers) {
        Ok(Html(html::render_transfers_table(&api_transfers, next_timestamp)).into_response())
    } else {
        Ok(Json(TransfersResponse {
            transfers: api_transfers,
            next_timestamp,
        })
        .into_response())
    }
}

/// Get a single transfer by ID.
pub async fn get_transfer(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let transfer_id = parse_id(&id)?;

    let transfers = {
        let client = state.client.lock().await;
        client.lookup_transfers(&[transfer_id]).await?
    };

    let transfer = transfers
        .first()
        .ok_or_else(|| AppError::NotFound(format!("Transfer {} not found", id)))?;

    let api_transfer = ApiTransfer::from(transfer);

    if is_htmx_request(&headers) {
        Ok(Html(html::render_transfer_detail(&api_transfer)).into_response())
    } else {
        Ok(Json(api_transfer).into_response())
    }
}

/// Parse a hex ID string to u128.
fn parse_id(id: &str) -> Result<u128, AppError> {
    u128::from_str_radix(id, 16).map_err(|_| AppError::BadRequest(format!("Invalid ID: {}", id)))
}
