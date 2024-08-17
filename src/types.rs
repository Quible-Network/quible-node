use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Transaction {
    // TODO: include author
    // TODO: include nonce
    // TODO: include ECDSA signature
    pub events: Vec<Event>
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "name")]
pub enum Event {
    CreateQuirkle {
        // TODO: this should be a vector of Keccak256 hashes
        members: Vec<String>,
        proof_ttl: u64
    }
}
