use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BlockHeader {
    Version1 {
        previous_block_header_hash: [u8; 32],
        merkle_root: [u8; 32],

        #[serde(with = "postcard::fixint::le")]
        timestamp: u32,
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

    // pubkey script
    // P2PKH pubkey script: OP_DUP OP_PUSH(<pkh>) OP_EQUALVERIFY OP_CHECKSIGVERIFY
    // P2PKH sig script: OP_PUSH(<sig>) OP_PUSH(<pkh>)
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionOutpoint {
    pub txid: [u8; 32],

    #[serde(with = "postcard::fixint::le")]
    pub index: u32,
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
        permit_index: u32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectIdentifier {
    pub raw: [u8; 32],
    pub mode: ObjectMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
pub enum Transaction {
    Version1 {
        inputs: Vec<TransactionInput>,
        outputs: Vec<TransactionOutput>,

        #[serde(with = "postcard::fixint::le")]
        locktime: u32,
    },
}
