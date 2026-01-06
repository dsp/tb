//! TigerBeetle protocol commands and operations.

/// VSR Command types.
///
/// These are the message types in the Viewstamped Replication protocol.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub enum Command {
    /// Reserved/invalid command (default).
    #[default]
    Reserved = 0,
    /// Replica-to-replica ping for liveness detection.
    Ping = 1,
    /// Replica-to-replica pong response.
    Pong = 2,
    /// Client-to-replica ping for connection keepalive.
    PingClient = 3,
    /// Replica-to-client pong response.
    PongClient = 4,
    /// Client request message.
    Request = 5,
    /// Leader prepare message to followers.
    Prepare = 6,
    /// Follower acknowledgment of prepare.
    PrepareOk = 7,
    /// Reply to client request.
    Reply = 8,
    /// Commit notification from leader.
    Commit = 9,
    /// Initiate view change protocol.
    StartViewChange = 10,
    /// View change proposal with log state.
    DoViewChange = 11,
    // 12 is deprecated
    /// Request to start a new view.
    RequestStartView = 13,
    /// Request message headers from peer.
    RequestHeaders = 14,
    /// Request specific prepare message.
    RequestPrepare = 15,
    /// Request specific reply message.
    RequestReply = 16,
    /// Response containing message headers.
    Headers = 17,
    /// Client eviction notification.
    Eviction = 18,
    /// Request storage blocks from peer.
    RequestBlocks = 19,
    /// Response containing storage block.
    Block = 20,
    // 21, 22, 23 are deprecated
    /// Announce new view to cluster.
    StartView = 24,
}

impl Command {
    /// Returns true if this is a client command (not replica-to-replica).
    pub fn is_client_command(self) -> bool {
        matches!(self, Command::Request | Command::PingClient)
    }
}

impl TryFrom<u8> for Command {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Command::Reserved),
            1 => Ok(Command::Ping),
            2 => Ok(Command::Pong),
            3 => Ok(Command::PingClient),
            4 => Ok(Command::PongClient),
            5 => Ok(Command::Request),
            6 => Ok(Command::Prepare),
            7 => Ok(Command::PrepareOk),
            8 => Ok(Command::Reply),
            9 => Ok(Command::Commit),
            10 => Ok(Command::StartViewChange),
            11 => Ok(Command::DoViewChange),
            13 => Ok(Command::RequestStartView),
            14 => Ok(Command::RequestHeaders),
            15 => Ok(Command::RequestPrepare),
            16 => Ok(Command::RequestReply),
            17 => Ok(Command::Headers),
            18 => Ok(Command::Eviction),
            19 => Ok(Command::RequestBlocks),
            20 => Ok(Command::Block),
            24 => Ok(Command::StartView),
            _ => Err(value),
        }
    }
}

/// VSR operations reserved boundary.
pub const VSR_OPERATIONS_RESERVED: u8 = 128;

/// State machine operations.
///
/// Operations < 128 are reserved for VSR protocol operations.
/// Operations >= 128 are user/state-machine operations.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub enum Operation {
    // VSR reserved operations (< 128)
    /// Reserved/invalid operation (default).
    #[default]
    Reserved = 0,
    /// Root operation for bootstrap.
    Root = 1,
    /// Register a new client session.
    Register = 2,
    /// Reconfigure cluster membership.
    Reconfigure = 3,
    /// Periodic pulse for time-based operations.
    Pulse = 4,
    /// Upgrade cluster to new version.
    Upgrade = 5,
    /// No-op for log compaction.
    Noop = 6,

    // TigerBeetle state machine operations (>= 128)
    /// Create accounts (batch).
    CreateAccounts = 138,
    /// Create transfers (batch).
    CreateTransfers = 139,
    /// Lookup accounts by ID (batch).
    LookupAccounts = 140,
    /// Lookup transfers by ID (batch).
    LookupTransfers = 141,
    /// Get transfers for an account (single filter).
    GetAccountTransfers = 142,
    /// Get balance history for an account (single filter).
    GetAccountBalances = 143,
    /// Query accounts (single filter).
    QueryAccounts = 144,
    /// Query transfers (single filter).
    QueryTransfers = 145,
}

impl Operation {
    /// Returns true if this is a VSR reserved operation.
    pub fn is_vsr_reserved(self) -> bool {
        (self as u8) < VSR_OPERATIONS_RESERVED
    }

    /// Returns true if this operation takes batched input.
    pub fn is_batchable(self) -> bool {
        matches!(
            self,
            Operation::CreateAccounts
                | Operation::CreateTransfers
                | Operation::LookupAccounts
                | Operation::LookupTransfers
        )
    }

    /// Returns true if this operation uses multi-batch encoding.
    ///
    /// Multi-batch encoding wraps the request body with a trailer containing
    /// batch count and element counts. All TigerBeetle state machine operations
    /// use this format.
    pub fn is_multi_batch(self) -> bool {
        matches!(
            self,
            Operation::CreateAccounts
                | Operation::CreateTransfers
                | Operation::LookupAccounts
                | Operation::LookupTransfers
                | Operation::GetAccountTransfers
                | Operation::GetAccountBalances
                | Operation::QueryAccounts
                | Operation::QueryTransfers
        )
    }
}

impl TryFrom<u8> for Operation {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Operation::Reserved),
            1 => Ok(Operation::Root),
            2 => Ok(Operation::Register),
            3 => Ok(Operation::Reconfigure),
            4 => Ok(Operation::Pulse),
            5 => Ok(Operation::Upgrade),
            6 => Ok(Operation::Noop),
            138 => Ok(Operation::CreateAccounts),
            139 => Ok(Operation::CreateTransfers),
            140 => Ok(Operation::LookupAccounts),
            141 => Ok(Operation::LookupTransfers),
            142 => Ok(Operation::GetAccountTransfers),
            143 => Ok(Operation::GetAccountBalances),
            144 => Ok(Operation::QueryAccounts),
            145 => Ok(Operation::QueryTransfers),
            _ => Err(value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_values() {
        assert_eq!(Command::Reserved as u8, 0);
        assert_eq!(Command::Request as u8, 5);
        assert_eq!(Command::Reply as u8, 8);
        assert_eq!(Command::PingClient as u8, 3);
        assert_eq!(Command::PongClient as u8, 4);
        assert_eq!(Command::Eviction as u8, 18);
    }

    #[test]
    fn test_operation_values() {
        assert_eq!(Operation::Reserved as u8, 0);
        assert_eq!(Operation::Register as u8, 2);
        assert_eq!(Operation::CreateAccounts as u8, 138);
        assert_eq!(Operation::CreateTransfers as u8, 139);
        assert_eq!(Operation::LookupAccounts as u8, 140);
        assert_eq!(Operation::LookupTransfers as u8, 141);
        assert_eq!(Operation::GetAccountTransfers as u8, 142);
        assert_eq!(Operation::GetAccountBalances as u8, 143);
        assert_eq!(Operation::QueryAccounts as u8, 144);
        assert_eq!(Operation::QueryTransfers as u8, 145);
    }

    #[test]
    fn test_operation_is_vsr_reserved() {
        assert!(Operation::Reserved.is_vsr_reserved());
        assert!(Operation::Register.is_vsr_reserved());
        assert!(!Operation::CreateAccounts.is_vsr_reserved());
    }

    #[test]
    fn test_operation_is_batchable() {
        assert!(Operation::CreateAccounts.is_batchable());
        assert!(Operation::CreateTransfers.is_batchable());
        assert!(Operation::LookupAccounts.is_batchable());
        assert!(!Operation::GetAccountTransfers.is_batchable());
        assert!(!Operation::QueryAccounts.is_batchable());
    }

    #[test]
    fn test_operation_is_multi_batch() {
        // All state machine operations use multi-batch encoding
        assert!(Operation::CreateAccounts.is_multi_batch());
        assert!(Operation::CreateTransfers.is_multi_batch());
        assert!(Operation::LookupAccounts.is_multi_batch());
        assert!(Operation::LookupTransfers.is_multi_batch());
        assert!(Operation::GetAccountTransfers.is_multi_batch());
        assert!(Operation::GetAccountBalances.is_multi_batch());
        assert!(Operation::QueryAccounts.is_multi_batch());
        assert!(Operation::QueryTransfers.is_multi_batch());
        // VSR operations don't use multi-batch
        assert!(!Operation::Register.is_multi_batch());
        assert!(!Operation::Reserved.is_multi_batch());
    }

    #[test]
    fn test_command_try_from() {
        assert_eq!(Command::try_from(5), Ok(Command::Request));
        assert_eq!(Command::try_from(8), Ok(Command::Reply));
        assert_eq!(Command::try_from(12), Err(12)); // deprecated
    }

    #[test]
    fn test_operation_try_from() {
        assert_eq!(Operation::try_from(2), Ok(Operation::Register));
        assert_eq!(Operation::try_from(138), Ok(Operation::CreateAccounts));
        assert_eq!(Operation::try_from(100), Err(100)); // unknown
    }
}
