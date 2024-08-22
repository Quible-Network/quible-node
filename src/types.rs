use bls_signatures::{Serialize as BlsSerialize, Signature};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct QuirkleSignature {
    pub bls_signature: Signature,
}

impl Serialize for QuirkleSignature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let bytes = &self.bls_signature.as_bytes();
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

// TODO: figure out https://stackoverflow.com/a/46755370
impl<'de> Deserialize<'de> for QuirkleSignature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: &str = Deserialize::deserialize(deserializer)?;

        // TODO: do this more efficiently because we're expecting only 96 bytes
        let byte_vec: Vec<u8> = (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
            .collect::<Result<Vec<u8>, std::num::ParseIntError>>()
            .map_err(|e| serde::de::Error::custom(e))?;

        let byte_array: [u8; 96] = byte_vec.try_into().unwrap();

        let g2_affine = bls12_381::G2Affine::from_compressed(&byte_array).unwrap();

        Ok(QuirkleSignature {
            bls_signature: Signature::from(g2_affine),
        })
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct QuirkleProof {
    pub quirkle_root: String,
    pub member_address: String,
    pub expires_at: u64,
    pub signature: QuirkleSignature,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Transaction {
    // TODO: include author
    // TODO: include nonce
    // TODO: include ECDSA signature
    pub events: Vec<Event>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "name")]
pub enum Event {
    CreateQuirkle {
        // TODO: this should be a vector of Keccak256 hashes
        members: Vec<String>,
        proof_ttl: u64,
    },
}
