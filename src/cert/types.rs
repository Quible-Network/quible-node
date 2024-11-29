use alloy_primitives::{FixedBytes, B256};
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use sha3::{Digest, Keccak256};

use crate::{quible_ecdsa_utils::sign_message, tx::types::Hashable};

pub trait Signable {
    fn sign(&self, secret: [u8; 32]) -> anyhow::Result<QuibleSignature>;
}

#[derive(Clone)]
pub struct QuibleSignature {
    pub raw: [u8; 65],
}

impl std::fmt::Debug for QuibleSignature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            &self
                .raw
                .iter()
                .map(|byte| format!("{:02x}", byte))
                .collect::<String>()
        )
    }
}

impl Serialize for QuibleSignature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let bytes = &self.raw;
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

impl<'de> Deserialize<'de> for QuibleSignature {
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

        let byte_array: [u8; 65] = byte_vec
            .try_into()
            .map_err(|e| serde::de::Error::custom(format!("{:#?}", e)))?;

        Ok(QuibleSignature { raw: byte_array })
    }
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateSigningRequestDetails {
    pub object_id: [u8; 32],
    pub claim: Vec<u8>,

    #[serde_as(as = "DisplayFromStr")]
    pub expires_at: u64,
}

impl Hashable for CertificateSigningRequestDetails {
    fn hash(&self) -> anyhow::Result<[u8; 32]> {
        let mut hasher = Keccak256::new();
        postcard::to_io(&self, &mut hasher)?;
        let hash_vec = hasher.finalize();
        hash_vec
            .as_slice()
            .try_into()
            .map_err(|_| anyhow!("failed to convert hash slice to 32 bytes"))
    }

    fn hash_eip191(&self) -> anyhow::Result<[u8; 32]> {
        panic!("not implemented")
    }
}

impl Signable for CertificateSigningRequestDetails {
    fn sign(&self, secret: [u8; 32]) -> anyhow::Result<QuibleSignature> {
        let hash = self.hash()?;

        let signature_bytes = sign_message(B256::from_slice(&secret), FixedBytes::new(hash))
            .map_err(|err| anyhow!(err))?;

        Ok(QuibleSignature {
            raw: signature_bytes,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedCertificate {
    pub details: CertificateSigningRequestDetails,
    pub signature: QuibleSignature,
}
