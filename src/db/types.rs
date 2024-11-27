use crate::tx::types::{BlockHeader, Transaction, TransactionOutput};
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionOutputRow {
    pub id: SurrealID,
    pub transaction_hash: String,
    pub output_index: u64,
    pub output: TransactionOutput,
    pub owner: String,
    pub spent: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectRow {
    pub id: SurrealID,
    pub object_id: String,
    pub cert_ttl: u64,
    pub claims: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntermediateFaucetOutputRow {
    pub id: SurrealID,
    pub transaction_hash_hex: String,
    pub output_index: u64,
    pub owner_signing_key_hex: String,
}
