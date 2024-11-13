use serde::{Deserialize, Serialize};
use crate::tx::types::{BlockHeader, Transaction};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurrealID(pub surrealdb::sql::Thing);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingTransactionRow {
    pub id: SurrealID,
    pub hash: String,
    // pub hash: surrealdb::sql::Bytes,
    // pub hash: TransactionHash,
    pub data: Transaction,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackerPing {
    pub peer_id: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockRow {
    pub id: SurrealID,
    pub hash: String,
    pub header: BlockHeader,
    pub height: u64,
    pub transactions: Vec<([u8; 32], Transaction)>,
}
