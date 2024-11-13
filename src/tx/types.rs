use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};

pub trait Hashable {
    fn hash(&self) -> anyhow::Result<[u8; 32]>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "version", content = "data")]
pub enum BlockHeader {
    Version1 {
        previous_block_header_hash: [u8; 32],
        merkle_root: [u8; 32],

        #[serde(with = "postcard::fixint::le")]
        timestamp: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionOpCode {
    // general
    Push { data: Vec<u8> },

    // pubkey script Pay-to-Address (P2A)
    // P2A pubkey script: OP_DUP OP_PUSH(<address>) OP_EQUALVERIFY OP_CHECKSIGVERIFY
    // P2A sig script: OP_PUSH(<sig>) OP_PUSH(<address>)
    CheckSigVerify,
    Dup,
    EqualVerify,

    // unspendable script opcodes
    Insert { data: Vec<u8> },
    Delete { data: Vec<u8> },
    NewSet,
    CommitSet, // TODO: think about how to use a psuedo-spendable UTXO
               //       where you have to provide a valid signature script
               //       before you can commit changes to the set
               //       before you can commit changes to the set
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransactionOutpoint {
    pub txid: [u8; 32],

    #[serde(with = "postcard::fixint::le")]
    pub index: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionInput {
    pub outpoint: TransactionOutpoint,
    pub signature_script: Vec<TransactionOpCode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ObjectMode {
    Fresh,
    Existing {
        #[serde(with = "postcard::fixint::le")]
        permit_index: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectIdentifier {
    pub raw: [u8; 32],
    pub mode: ObjectMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TransactionOutput {
    Value {
        #[serde(with = "postcard::fixint::le")]
        value: u64,
        pubkey_script: Vec<TransactionOpCode>,
    },

    Object {
        object_id: ObjectIdentifier,
        data_script: Vec<TransactionOpCode>,
        pubkey_script: Vec<TransactionOpCode>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "version", content = "data")]
pub enum Transaction {
    Version1 {
        inputs: Vec<TransactionInput>,
        outputs: Vec<TransactionOutput>,

        #[serde(with = "postcard::fixint::le")]
        locktime: u64,
    },
}

impl Hashable for Transaction {
    fn hash(&self) -> anyhow::Result<[u8; 32]> {
        let mut transaction_data_hasher = Keccak256::new();
        postcard::to_io(&self, &mut transaction_data_hasher)?;
        let transaction_hash_vec = transaction_data_hasher.finalize();
        transaction_hash_vec
            .as_slice()
            .try_into()
            .map_err(|_| anyhow!("failed to convert hash slice to 32 bytes"))
    }
}

impl Hashable for BlockHeader {
    fn hash(&self) -> anyhow::Result<[u8; 32]> {
        let mut hasher = Keccak256::new();
        postcard::to_io(&self, &mut hasher)?;
        let hash_vec = hasher.finalize();
        hash_vec
            .as_slice()
            .try_into()
            .map_err(|_| anyhow!("failed to convert hash slice to 32 bytes"))
    }
}
