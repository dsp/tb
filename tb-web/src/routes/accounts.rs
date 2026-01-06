//! Account route handlers.

use crate::api::{
    AccountsResponse, ApiAccount, ApiAccountBalance, ApiTransfer, BalancesResponse,
    TransfersResponse,
};
use crate::error::AppError;
use crate::html;
use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::response::{Html, IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use std::sync::Arc;
use tb_rs::{AccountFilter, AccountFilterFlags, QueryFilter, QueryFilterFlags};

/// Check if request is from HTMX.
fn is_htmx_request(headers: &HeaderMap) -> bool {
    headers.contains_key("hx-request")
}

/// Query parameters for listing accounts.
#[derive(Debug, Deserialize)]
pub struct ListAccountsParams {
    /// Filter by ledger.
    pub ledger: Option<u32>,
    /// Filter by code.
    pub code: Option<u16>,
    /// Maximum number of results.
    #[serde(default = "default_limit")]
    pub limit: u32,
    /// Pagination: start after this timestamp.
    pub after_timestamp: Option<u64>,
}

fn default_limit() -> u32 {
    100
}

/// List accounts with optional filters.
pub async fn list_accounts(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(params): Query<ListAccountsParams>,
) -> Result<Response, AppError> {
    let filter = QueryFilter {
        user_data_128: 0,
        user_data_64: 0,
        user_data_32: 0,
        ledger: params.ledger.unwrap_or(0),
        code: params.code.unwrap_or(0),
        timestamp_min: params.after_timestamp.map(|t| t + 1).unwrap_or(0),
        timestamp_max: 0,
        limit: params.limit,
        flags: QueryFilterFlags::empty(),
        reserved: [0; 6],
    };

    let accounts = {
        let client = state.client.lock().await;
        client.query_accounts(filter).await?
    };

    let next_timestamp = accounts.last().map(|a| a.timestamp);
    let api_accounts: Vec<ApiAccount> = accounts.iter().map(ApiAccount::from).collect();

    if is_htmx_request(&headers) {
        Ok(Html(html::render_accounts_table(&api_accounts, next_timestamp)).into_response())
    } else {
        Ok(Json(AccountsResponse {
            accounts: api_accounts,
            next_timestamp,
        })
        .into_response())
    }
}

/// Get a single account by ID.
pub async fn get_account(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let account_id = parse_id(&id)?;

    let accounts = {
        let client = state.client.lock().await;
        client.lookup_accounts(&[account_id]).await?
    };

    let account = accounts
        .first()
        .ok_or_else(|| AppError::NotFound(format!("Account {} not found", id)))?;

    let api_account = ApiAccount::from(account);

    if is_htmx_request(&headers) {
        Ok(Html(html::render_account_detail(&api_account)).into_response())
    } else {
        Ok(Json(api_account).into_response())
    }
}

/// Query parameters for account transfers.
#[derive(Debug, Deserialize)]
pub struct AccountTransfersParams {
    /// Include debit transfers.
    #[serde(default = "default_true")]
    pub debits: bool,
    /// Include credit transfers.
    #[serde(default = "default_true")]
    pub credits: bool,
    /// Return in reverse chronological order.
    #[serde(default)]
    pub reversed: bool,
    /// Maximum number of results.
    #[serde(default = "default_limit")]
    pub limit: u32,
    /// Pagination: start after this timestamp.
    pub after_timestamp: Option<u64>,
}

fn default_true() -> bool {
    true
}

/// Get transfers for an account.
pub async fn get_account_transfers(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(params): Query<AccountTransfersParams>,
) -> Result<Response, AppError> {
    let account_id = parse_id(&id)?;

    let mut flags = AccountFilterFlags::empty();
    if params.debits {
        flags |= AccountFilterFlags::DEBITS;
    }
    if params.credits {
        flags |= AccountFilterFlags::CREDITS;
    }
    if params.reversed {
        flags |= AccountFilterFlags::REVERSED;
    }

    let filter = AccountFilter {
        account_id,
        user_data_128: 0,
        user_data_64: 0,
        user_data_32: 0,
        code: 0,
        timestamp_min: params.after_timestamp.map(|t| t + 1).unwrap_or(0),
        timestamp_max: 0,
        limit: params.limit,
        flags,
        reserved: [0; 58],
    };

    let transfers = {
        let client = state.client.lock().await;
        client.get_account_transfers(filter).await?
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

/// Query parameters for account balances.
#[derive(Debug, Deserialize)]
pub struct AccountBalancesParams {
    /// Maximum number of results.
    #[serde(default = "default_limit")]
    pub limit: u32,
    /// Return in reverse chronological order.
    #[serde(default)]
    pub reversed: bool,
}

/// Get balance history for an account.
pub async fn get_account_balances(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(params): Query<AccountBalancesParams>,
) -> Result<Json<BalancesResponse>, AppError> {
    let account_id = parse_id(&id)?;

    let mut flags = AccountFilterFlags::DEBITS | AccountFilterFlags::CREDITS;
    if params.reversed {
        flags |= AccountFilterFlags::REVERSED;
    }

    let filter = AccountFilter {
        account_id,
        user_data_128: 0,
        user_data_64: 0,
        user_data_32: 0,
        code: 0,
        timestamp_min: 0,
        timestamp_max: 0,
        limit: params.limit,
        flags,
        reserved: [0; 58],
    };

    let balances = {
        let client = state.client.lock().await;
        client.get_account_balances(filter).await?
    };

    let api_balances: Vec<ApiAccountBalance> =
        balances.iter().map(ApiAccountBalance::from).collect();

    Ok(Json(BalancesResponse {
        balances: api_balances,
    }))
}

/// Parse a hex ID string to u128.
fn parse_id(id: &str) -> Result<u128, AppError> {
    u128::from_str_radix(id, 16).map_err(|_| AppError::BadRequest(format!("Invalid ID: {}", id)))
}
