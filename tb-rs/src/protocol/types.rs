//! TigerBeetle protocol data types.
//!
//! These types match the exact byte layout of the TigerBeetle wire protocol.
//! All types use `#[repr(C)]` to ensure C-compatible memory layout.

use bitflags::bitflags;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

// Note: Types containing bitflags (Account, Transfer, filters) cannot use zerocopy
// derives because bitflags! generates internal types without those traits.
// The serialization code for these types uses safe patterns (slice::from_raw_parts
// on #[repr(C)] types), and deserialization uses read_unaligned which handles
// alignment correctly.

/// TigerBeetle Account (128 bytes).
///
/// Accounts are the fundamental unit of accounting in TigerBeetle.
/// They track debits and credits with pending and posted balances.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Account {
    /// Unique identifier for the account.
    pub id: u128,
    /// Sum of pending debit transfers.
    pub debits_pending: u128,
    /// Sum of posted debit transfers.
    pub debits_posted: u128,
    /// Sum of pending credit transfers.
    pub credits_pending: u128,
    /// Sum of posted credit transfers.
    pub credits_posted: u128,
    /// Opaque user data for external linking (128-bit indexed).
    pub user_data_128: u128,
    /// Opaque user data for external linking (64-bit indexed).
    pub user_data_64: u64,
    /// Opaque user data for external linking (32-bit indexed).
    pub user_data_32: u32,
    /// Reserved for accounting policy primitives.
    pub reserved: u32,
    /// The ledger this account belongs to.
    pub ledger: u32,
    /// Chart of accounts code describing the account type.
    pub code: u16,
    /// Account flags.
    pub flags: AccountFlags,
    /// Timestamp when the account was created (set by server).
    pub timestamp: u64,
}

const _: () = assert!(std::mem::size_of::<Account>() == 128);

bitflags! {
    /// Flags for Account configuration.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
    pub struct AccountFlags: u16 {
        /// Link this account with the next in a chain.
        const LINKED = 1 << 0;
        /// Enforce that debits do not exceed credits.
        const DEBITS_MUST_NOT_EXCEED_CREDITS = 1 << 1;
        /// Enforce that credits do not exceed debits.
        const CREDITS_MUST_NOT_EXCEED_DEBITS = 1 << 2;
        /// Enable balance history for this account.
        const HISTORY = 1 << 3;
        /// Mark this account as imported (for data migration).
        const IMPORTED = 1 << 4;
        /// Mark this account as closed.
        const CLOSED = 1 << 5;
    }
}

/// TigerBeetle Transfer (128 bytes).
///
/// Transfers move value between accounts by debiting one and crediting another.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Transfer {
    /// Unique identifier for the transfer.
    pub id: u128,
    /// Account ID to debit.
    pub debit_account_id: u128,
    /// Account ID to credit.
    pub credit_account_id: u128,
    /// Amount to transfer.
    pub amount: u128,
    /// ID of pending transfer to post or void (0 if not applicable).
    pub pending_id: u128,
    /// Opaque user data for external linking (128-bit indexed).
    pub user_data_128: u128,
    /// Opaque user data for external linking (64-bit indexed).
    pub user_data_64: u64,
    /// Opaque user data for external linking (32-bit indexed).
    pub user_data_32: u32,
    /// Timeout in seconds for pending transfers.
    pub timeout: u32,
    /// The ledger this transfer operates on.
    pub ledger: u32,
    /// Chart of accounts code describing the transfer type.
    pub code: u16,
    /// Transfer flags.
    pub flags: TransferFlags,
    /// Timestamp when the transfer was created (set by server).
    pub timestamp: u64,
}

const _: () = assert!(std::mem::size_of::<Transfer>() == 128);

bitflags! {
    /// Flags for Transfer configuration.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
    pub struct TransferFlags: u16 {
        /// Link this transfer with the next in a chain.
        const LINKED = 1 << 0;
        /// Create a pending (two-phase) transfer.
        const PENDING = 1 << 1;
        /// Post a pending transfer.
        const POST_PENDING_TRANSFER = 1 << 2;
        /// Void a pending transfer.
        const VOID_PENDING_TRANSFER = 1 << 3;
        /// Balance the debit side.
        const BALANCING_DEBIT = 1 << 4;
        /// Balance the credit side.
        const BALANCING_CREDIT = 1 << 5;
        /// Close the debit account after this transfer.
        const CLOSING_DEBIT = 1 << 6;
        /// Close the credit account after this transfer.
        const CLOSING_CREDIT = 1 << 7;
        /// Mark this transfer as imported (for data migration).
        const IMPORTED = 1 << 8;
    }
}

/// Account balance at a point in time (128 bytes).
///
/// Used for historical balance queries.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AccountBalance {
    /// Pending debits at this timestamp.
    pub debits_pending: u128,
    /// Posted debits at this timestamp.
    pub debits_posted: u128,
    /// Pending credits at this timestamp.
    pub credits_pending: u128,
    /// Posted credits at this timestamp.
    pub credits_posted: u128,
    /// Timestamp of this balance snapshot.
    pub timestamp: u64,
    /// Reserved for future use.
    pub reserved: [u8; 56],
}

impl Default for AccountBalance {
    fn default() -> Self {
        Self {
            debits_pending: 0,
            debits_posted: 0,
            credits_pending: 0,
            credits_posted: 0,
            timestamp: 0,
            reserved: [0; 56],
        }
    }
}

const _: () = assert!(std::mem::size_of::<AccountBalance>() == 128);

/// Filter for account-related queries (128 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct AccountFilter {
    /// Account ID to query.
    pub account_id: u128,
    /// Filter by user_data_128 (0 for no filter).
    pub user_data_128: u128,
    /// Filter by user_data_64 (0 for no filter).
    pub user_data_64: u64,
    /// Filter by user_data_32 (0 for no filter).
    pub user_data_32: u32,
    /// Filter by code (0 for no filter).
    pub code: u16,
    /// Reserved for future use.
    pub reserved: [u8; 58],
    /// Minimum timestamp (inclusive, 0 for no filter).
    pub timestamp_min: u64,
    /// Maximum timestamp (inclusive, 0 for no filter).
    pub timestamp_max: u64,
    /// Maximum number of results.
    pub limit: u32,
    /// Query flags.
    pub flags: AccountFilterFlags,
}

impl Default for AccountFilter {
    fn default() -> Self {
        Self {
            account_id: 0,
            user_data_128: 0,
            user_data_64: 0,
            user_data_32: 0,
            code: 0,
            reserved: [0; 58],
            timestamp_min: 0,
            timestamp_max: 0,
            limit: 0,
            flags: AccountFilterFlags::empty(),
        }
    }
}

const _: () = assert!(std::mem::size_of::<AccountFilter>() == 128);

bitflags! {
    /// Flags for AccountFilter queries.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
    pub struct AccountFilterFlags: u32 {
        /// Include debit transfers.
        const DEBITS = 1 << 0;
        /// Include credit transfers.
        const CREDITS = 1 << 1;
        /// Return results in reverse order.
        const REVERSED = 1 << 2;
    }
}

/// Filter for general queries (64 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct QueryFilter {
    /// Filter by user_data_128 (0 for no filter).
    pub user_data_128: u128,
    /// Filter by user_data_64 (0 for no filter).
    pub user_data_64: u64,
    /// Filter by user_data_32 (0 for no filter).
    pub user_data_32: u32,
    /// Filter by ledger (0 for no filter).
    pub ledger: u32,
    /// Filter by code (0 for no filter).
    pub code: u16,
    /// Reserved for future use.
    pub reserved: [u8; 6],
    /// Minimum timestamp (inclusive, 0 for no filter).
    pub timestamp_min: u64,
    /// Maximum timestamp (inclusive, 0 for no filter).
    pub timestamp_max: u64,
    /// Maximum number of results.
    pub limit: u32,
    /// Query flags.
    pub flags: QueryFilterFlags,
}

const _: () = assert!(std::mem::size_of::<QueryFilter>() == 64);

bitflags! {
    /// Flags for QueryFilter queries.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
    pub struct QueryFilterFlags: u32 {
        /// Return results in reverse order.
        const REVERSED = 1 << 0;
    }
}

/// Result of a create_accounts operation (8 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CreateAccountsResult {
    /// Index of the account in the request batch.
    pub index: u32,
    /// Result code for this account.
    pub result: CreateAccountResult,
}

const _: () = assert!(std::mem::size_of::<CreateAccountsResult>() == 8);

/// Result of a create_transfers operation (8 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CreateTransfersResult {
    /// Index of the transfer in the request batch.
    pub index: u32,
    /// Result code for this transfer.
    pub result: CreateTransferResult,
}

const _: () = assert!(std::mem::size_of::<CreateTransfersResult>() == 8);

/// Register request body (256 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct RegisterRequest {
    /// Batch size limit (0 for clients, >0 for prepares).
    pub batch_size_limit: u32,
    /// Reserved for future use.
    pub reserved: [u8; 252],
}

impl Default for RegisterRequest {
    fn default() -> Self {
        Self {
            batch_size_limit: 0,
            reserved: [0; 252],
        }
    }
}

const _: () = assert!(std::mem::size_of::<RegisterRequest>() == 256);

/// Register result body (64 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct RegisterResult {
    /// Maximum body size for requests.
    pub batch_size_limit: u32,
    /// Reserved for future use.
    pub reserved: [u8; 60],
}

impl Default for RegisterResult {
    fn default() -> Self {
        Self {
            batch_size_limit: 0,
            reserved: [0; 60],
        }
    }
}

const _: () = assert!(std::mem::size_of::<RegisterResult>() == 64);

/// Create account result codes.
///
/// These match the exact values from the TigerBeetle protocol.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CreateAccountResult {
    /// Account created successfully.
    Ok = 0,
    /// A linked event in the batch failed, so this event was not applied.
    LinkedEventFailed = 1,
    /// A linked event chain was not closed properly.
    LinkedEventChainOpen = 2,
    /// The timestamp field must be zero (server assigns timestamps).
    TimestampMustBeZero = 3,
    /// A reserved field was set to a non-zero value.
    ReservedField = 4,
    /// A reserved flag was set.
    ReservedFlag = 5,
    /// Account ID must not be zero.
    IdMustNotBeZero = 6,
    /// Account ID must not be `u128::MAX`.
    IdMustNotBeIntMax = 7,
    /// Mutually exclusive flags were set together.
    FlagsAreMutuallyExclusive = 8,
    /// `debits_pending` must be zero on creation.
    DebitsPendingMustBeZero = 9,
    /// `debits_posted` must be zero on creation.
    DebitsPostedMustBeZero = 10,
    /// `credits_pending` must be zero on creation.
    CreditsPendingMustBeZero = 11,
    /// `credits_posted` must be zero on creation.
    CreditsPostedMustBeZero = 12,
    /// Ledger must not be zero.
    LedgerMustNotBeZero = 13,
    /// Code must not be zero.
    CodeMustNotBeZero = 14,
    /// Account exists with different flags.
    ExistsWithDifferentFlags = 15,
    /// Account exists with different `user_data_128`.
    ExistsWithDifferentUserData128 = 16,
    /// Account exists with different `user_data_64`.
    ExistsWithDifferentUserData64 = 17,
    /// Account exists with different `user_data_32`.
    ExistsWithDifferentUserData32 = 18,
    /// Account exists with different ledger.
    ExistsWithDifferentLedger = 19,
    /// Account exists with different code.
    ExistsWithDifferentCode = 20,
    /// Account already exists (idempotent success).
    Exists = 21,
    /// Expected an imported event but `IMPORTED` flag not set.
    ImportedEventExpected = 22,
    /// `IMPORTED` flag set but not in import mode.
    ImportedEventNotExpected = 23,
    /// Imported event timestamp is out of valid range.
    ImportedEventTimestampOutOfRange = 24,
    /// Imported event timestamp must not advance beyond current.
    ImportedEventTimestampMustNotAdvance = 25,
    /// Imported event timestamp must not regress.
    ImportedEventTimestampMustNotRegress = 26,
}

/// Create transfer result codes.
///
/// These match the exact values from the TigerBeetle protocol.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CreateTransferResult {
    /// Transfer created successfully.
    Ok = 0,
    /// A linked event in the batch failed, so this event was not applied.
    LinkedEventFailed = 1,
    /// A linked event chain was not closed properly.
    LinkedEventChainOpen = 2,
    /// The timestamp field must be zero (server assigns timestamps).
    TimestampMustBeZero = 3,
    /// A reserved flag was set.
    ReservedFlag = 4,
    /// Transfer ID must not be zero.
    IdMustNotBeZero = 5,
    /// Transfer ID must not be `u128::MAX`.
    IdMustNotBeIntMax = 6,
    /// Mutually exclusive flags were set together.
    FlagsAreMutuallyExclusive = 7,
    /// Debit account ID must not be zero.
    DebitAccountIdMustNotBeZero = 8,
    /// Debit account ID must not be `u128::MAX`.
    DebitAccountIdMustNotBeIntMax = 9,
    /// Credit account ID must not be zero.
    CreditAccountIdMustNotBeZero = 10,
    /// Credit account ID must not be `u128::MAX`.
    CreditAccountIdMustNotBeIntMax = 11,
    /// Debit and credit accounts must be different.
    AccountsMustBeDifferent = 12,
    /// `pending_id` must be zero for non-posting/voiding transfers.
    PendingIdMustBeZero = 13,
    /// `pending_id` must not be zero for posting/voiding transfers.
    PendingIdMustNotBeZero = 14,
    /// `pending_id` must not be `u128::MAX`.
    PendingIdMustNotBeIntMax = 15,
    /// `pending_id` must be different from the transfer ID.
    PendingIdMustBeDifferent = 16,
    /// Timeout is only valid for pending transfers.
    TimeoutReservedForPendingTransfer = 17,
    // 18 is deprecated (amount_must_not_be_zero)
    /// Ledger must not be zero.
    LedgerMustNotBeZero = 19,
    /// Code must not be zero.
    CodeMustNotBeZero = 20,
    /// Debit account not found.
    DebitAccountNotFound = 21,
    /// Credit account not found.
    CreditAccountNotFound = 22,
    /// Debit and credit accounts must have the same ledger.
    AccountsMustHaveTheSameLedger = 23,
    /// Transfer ledger must match the accounts' ledger.
    TransferMustHaveTheSameLedgerAsAccounts = 24,
    /// Referenced pending transfer not found.
    PendingTransferNotFound = 25,
    /// Referenced transfer is not pending.
    PendingTransferNotPending = 26,
    /// Pending transfer has different debit account.
    PendingTransferHasDifferentDebitAccountId = 27,
    /// Pending transfer has different credit account.
    PendingTransferHasDifferentCreditAccountId = 28,
    /// Pending transfer has different ledger.
    PendingTransferHasDifferentLedger = 29,
    /// Pending transfer has different code.
    PendingTransferHasDifferentCode = 30,
    /// Post amount exceeds pending transfer amount.
    ExceedsPendingTransferAmount = 31,
    /// Pending transfer has different amount (for void).
    PendingTransferHasDifferentAmount = 32,
    /// Pending transfer was already posted.
    PendingTransferAlreadyPosted = 33,
    /// Pending transfer was already voided.
    PendingTransferAlreadyVoided = 34,
    /// Pending transfer has expired.
    PendingTransferExpired = 35,
    /// Transfer exists with different flags.
    ExistsWithDifferentFlags = 36,
    /// Transfer exists with different debit account.
    ExistsWithDifferentDebitAccountId = 37,
    /// Transfer exists with different credit account.
    ExistsWithDifferentCreditAccountId = 38,
    /// Transfer exists with different amount.
    ExistsWithDifferentAmount = 39,
    /// Transfer exists with different `pending_id`.
    ExistsWithDifferentPendingId = 40,
    /// Transfer exists with different `user_data_128`.
    ExistsWithDifferentUserData128 = 41,
    /// Transfer exists with different `user_data_64`.
    ExistsWithDifferentUserData64 = 42,
    /// Transfer exists with different `user_data_32`.
    ExistsWithDifferentUserData32 = 43,
    /// Transfer exists with different timeout.
    ExistsWithDifferentTimeout = 44,
    /// Transfer exists with different code.
    ExistsWithDifferentCode = 45,
    /// Transfer already exists (idempotent success).
    Exists = 46,
    /// Transfer would overflow debit account's `debits_pending`.
    OverflowsDebitsPending = 47,
    /// Transfer would overflow credit account's `credits_pending`.
    OverflowsCreditsPending = 48,
    /// Transfer would overflow debit account's `debits_posted`.
    OverflowsDebitsPosted = 49,
    /// Transfer would overflow credit account's `credits_posted`.
    OverflowsCreditsPosted = 50,
    /// Transfer would overflow debit account's total debits.
    OverflowsDebits = 51,
    /// Transfer would overflow credit account's total credits.
    OverflowsCredits = 52,
    /// Transfer timeout would overflow.
    OverflowsTimeout = 53,
    /// Transfer exceeds credit account's available credits.
    ExceedsCredits = 54,
    /// Transfer exceeds debit account's available debits.
    ExceedsDebits = 55,
    /// Expected an imported event but `IMPORTED` flag not set.
    ImportedEventExpected = 56,
    /// `IMPORTED` flag set but not in import mode.
    ImportedEventNotExpected = 57,
    /// Imported event timestamp is out of valid range.
    ImportedEventTimestampOutOfRange = 58,
    /// Imported event timestamp must not advance beyond current.
    ImportedEventTimestampMustNotAdvance = 59,
    /// Imported event timestamp must not regress.
    ImportedEventTimestampMustNotRegress = 60,
    /// Imported event timestamp must postdate the debit account.
    ImportedEventTimestampMustPostdateDebitAccount = 61,
    /// Imported event timestamp must postdate the credit account.
    ImportedEventTimestampMustPostdateCreditAccount = 62,
    /// Imported event timeout must be zero.
    ImportedEventTimeoutMustBeZero = 63,
    /// Closing transfer must reference a pending transfer.
    ClosingTransferMustBePending = 64,
    /// Debit account is already closed.
    DebitAccountAlreadyClosed = 65,
    /// Credit account is already closed.
    CreditAccountAlreadyClosed = 66,
    /// Transfer exists with different ledger.
    ExistsWithDifferentLedger = 67,
    /// This ID was previously used in a failed transfer.
    IdAlreadyFailed = 68,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_size() {
        assert_eq!(std::mem::size_of::<Account>(), 128);
        assert_eq!(std::mem::align_of::<Account>(), 16);
    }

    #[test]
    fn test_transfer_size() {
        assert_eq!(std::mem::size_of::<Transfer>(), 128);
        assert_eq!(std::mem::align_of::<Transfer>(), 16);
    }

    #[test]
    fn test_account_balance_size() {
        assert_eq!(std::mem::size_of::<AccountBalance>(), 128);
    }

    #[test]
    fn test_account_filter_size() {
        assert_eq!(std::mem::size_of::<AccountFilter>(), 128);
    }

    #[test]
    fn test_query_filter_size() {
        assert_eq!(std::mem::size_of::<QueryFilter>(), 64);
    }

    #[test]
    fn test_create_accounts_result_size() {
        assert_eq!(std::mem::size_of::<CreateAccountsResult>(), 8);
    }

    #[test]
    fn test_create_transfers_result_size() {
        assert_eq!(std::mem::size_of::<CreateTransfersResult>(), 8);
    }

    #[test]
    fn test_register_request_size() {
        assert_eq!(std::mem::size_of::<RegisterRequest>(), 256);
    }

    #[test]
    fn test_register_result_size() {
        assert_eq!(std::mem::size_of::<RegisterResult>(), 64);
    }

    #[test]
    fn test_account_flags() {
        let flags = AccountFlags::LINKED | AccountFlags::HISTORY;
        assert_eq!(flags.bits(), 0b1001);
    }

    #[test]
    fn test_transfer_flags() {
        let flags = TransferFlags::PENDING | TransferFlags::LINKED;
        assert_eq!(flags.bits(), 0b11);
    }
}
