//! JSON-serializable API response types.
//!
//! u128 values are serialized as strings to avoid JavaScript precision issues.

use serde::Serialize;
use tb_rs::{Account, AccountBalance, Transfer};

/// Account response type.
#[derive(Debug, Serialize)]
pub struct ApiAccount {
    pub id: String,
    pub debits_pending: String,
    pub debits_posted: String,
    pub credits_pending: String,
    pub credits_posted: String,
    pub user_data_128: String,
    pub user_data_64: u64,
    pub user_data_32: u32,
    pub ledger: u32,
    pub code: u16,
    pub flags: u16,
    pub timestamp: u64,
}

impl From<&Account> for ApiAccount {
    fn from(a: &Account) -> Self {
        Self {
            id: format!("{:032x}", a.id),
            debits_pending: a.debits_pending.to_string(),
            debits_posted: a.debits_posted.to_string(),
            credits_pending: a.credits_pending.to_string(),
            credits_posted: a.credits_posted.to_string(),
            user_data_128: format!("{:032x}", a.user_data_128),
            user_data_64: a.user_data_64,
            user_data_32: a.user_data_32,
            ledger: a.ledger,
            code: a.code,
            flags: a.flags.bits(),
            timestamp: a.timestamp,
        }
    }
}

/// Transfer response type.
#[derive(Debug, Serialize)]
pub struct ApiTransfer {
    pub id: String,
    pub debit_account_id: String,
    pub credit_account_id: String,
    pub amount: String,
    pub pending_id: String,
    pub user_data_128: String,
    pub user_data_64: u64,
    pub user_data_32: u32,
    pub timeout: u32,
    pub ledger: u32,
    pub code: u16,
    pub flags: u16,
    pub timestamp: u64,
}

impl From<&Transfer> for ApiTransfer {
    fn from(t: &Transfer) -> Self {
        Self {
            id: format!("{:032x}", t.id),
            debit_account_id: format!("{:032x}", t.debit_account_id),
            credit_account_id: format!("{:032x}", t.credit_account_id),
            amount: t.amount.to_string(),
            pending_id: format!("{:032x}", t.pending_id),
            user_data_128: format!("{:032x}", t.user_data_128),
            user_data_64: t.user_data_64,
            user_data_32: t.user_data_32,
            timeout: t.timeout,
            ledger: t.ledger,
            code: t.code,
            flags: t.flags.bits(),
            timestamp: t.timestamp,
        }
    }
}

/// Account balance response type.
#[derive(Debug, Serialize)]
pub struct ApiAccountBalance {
    pub debits_pending: String,
    pub debits_posted: String,
    pub credits_pending: String,
    pub credits_posted: String,
    pub timestamp: u64,
}

impl From<&AccountBalance> for ApiAccountBalance {
    fn from(b: &AccountBalance) -> Self {
        Self {
            debits_pending: b.debits_pending.to_string(),
            debits_posted: b.debits_posted.to_string(),
            credits_pending: b.credits_pending.to_string(),
            credits_posted: b.credits_posted.to_string(),
            timestamp: b.timestamp,
        }
    }
}

/// Paginated accounts response.
#[derive(Debug, Serialize)]
pub struct AccountsResponse {
    pub accounts: Vec<ApiAccount>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_timestamp: Option<u64>,
}

/// Paginated transfers response.
#[derive(Debug, Serialize)]
pub struct TransfersResponse {
    pub transfers: Vec<ApiTransfer>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_timestamp: Option<u64>,
}

/// Account balances response.
#[derive(Debug, Serialize)]
pub struct BalancesResponse {
    pub balances: Vec<ApiAccountBalance>,
}

/// Health check response.
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub tb_connected: bool,
}
