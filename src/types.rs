use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

use crate::tx::types::{BlockHeader, Transaction, TransactionOutpoint};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurrealID(pub surrealdb::sql::Thing);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingTransactionRow {
    pub id: SurrealID,
    pub hash: String,

    // https://linear.app/quible/issue/QUI-99/use-surrealdb-bytes-type-for-storing-hashes
    // pub hash: surrealdb::sql::Bytes,
    pub data: Transaction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackerPing {
    pub peer_id: String,
    pub timestamp: u64,
}

#[derive(Clone)]
pub struct QuirkleSignature {
    pub ecdsa_signature_bytes: [u8; 65],
}

impl std::fmt::Debug for QuirkleSignature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            &self
                .ecdsa_signature_bytes
                .iter()
                .map(|byte| format!("{:02x}", byte))
                .collect::<String>()
        )
    }
}

// TODO: privatize members and use as_bytes() method instead
#[derive(Clone)]
pub struct QuirkleRoot {
    pub bytes: [u8; 32],
}

impl std::fmt::Debug for QuirkleRoot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            &self
                .bytes
                .iter()
                .map(|byte| format!("{:02x}", byte))
                .collect::<String>()
        )
    }
}

// TODO: privatize members and use as_bytes() method instead
#[derive(Clone)]
pub struct ECDSASignature {
    pub bytes: [u8; 65],
}

impl std::fmt::Debug for ECDSASignature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            &self
                .bytes
                .iter()
                .map(|byte| format!("{:02x}", byte))
                .collect::<String>()
        )
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HealthCheckResponse {
    pub status: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct QuirkleProof {
    pub quirkle_root: QuirkleRoot,
    pub member_address: String,
    pub expires_at: u64,
    pub signature: QuirkleSignature,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "name")]
pub enum Event {
    CreateQuirkle {
        // TODO(QUI-20): this should be a vector of Keccak256 hashes
        members: Vec<String>,
        proof_ttl: u64,
        slug: Option<String>,
    },
}

impl Serialize for QuirkleRoot {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let hash = format!(
            "0x{}",
            &self
                .bytes
                .iter()
                .map(|byte| format!("{:02x}", byte))
                .collect::<String>()
        );
        serializer.serialize_str(&hash)
    }
}

impl<'de> Deserialize<'de> for QuirkleRoot {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: &str = Deserialize::deserialize(deserializer)?;

        let byte_vec: Vec<u8> = (2..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
            .collect::<Result<Vec<u8>, std::num::ParseIntError>>()
            .map_err(|e| serde::de::Error::custom(e))?;

        let byte_array: [u8; 32] = byte_vec.try_into().unwrap();

        Ok(QuirkleRoot { bytes: byte_array })
    }
}

impl Serialize for QuirkleSignature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let bytes = &self.ecdsa_signature_bytes;
        let hash = format!(
            "{}",
            bytes
                .iter()
                .map(|byte| format!("{:02x}", byte))
                .collect::<String>()
        );
        serializer.serialize_str(&hash)
    }
}

impl<'de> Deserialize<'de> for QuirkleSignature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: &str = Deserialize::deserialize(deserializer)?;

        let byte_vec: Vec<u8> = (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
            .collect::<Result<Vec<u8>, std::num::ParseIntError>>()
            .map_err(|e| serde::de::Error::custom(e))?;

        let byte_array: [u8; 65] = byte_vec.try_into().unwrap();

        Ok(QuirkleSignature {
            ecdsa_signature_bytes: byte_array,
        })
    }
}

impl Serialize for ECDSASignature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let bytes = &self.bytes;
        let hash = format!(
            "0x{}",
            bytes
                .iter()
                .map(|byte| format!("{:02x}", byte))
                .collect::<String>()
        );
        serializer.serialize_str(&hash)
    }
}

impl<'de> Deserialize<'de> for ECDSASignature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;

        let byte_vec: Vec<u8> = (2..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
            .collect::<Result<Vec<u8>, std::num::ParseIntError>>()
            .map_err(|e| serde::de::Error::custom(e))?;

        let bytes: [u8; 65] = byte_vec.try_into().unwrap();

        Ok(ECDSASignature { bytes })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueOutputEntry {
    pub outpoint: TransactionOutpoint,
    pub value: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueOutputsPayload {
    pub total_value: u64,
    pub outputs: Vec<ValueOutputEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaucetOutputPayload {
    pub outpoint: TransactionOutpoint,
    pub value: u64,
    pub owner_signing_key: [u8; 32],
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeightPayload {
    #[serde_as(as = "DisplayFromStr")]
    pub height: u64,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockDetailsPayload {
    pub hash: [u8; 32],
    #[serde_as(as = "DisplayFromStr")]
    pub height: u64,
    pub header: BlockHeader,
    #[serde_as(as = "DisplayFromStr")]
    pub transaction_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutpointsPayload {
    pub outpoints: Vec<TransactionOutpoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimsPayload {
    pub claims: Vec<Vec<u8>>,
}
