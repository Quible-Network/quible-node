use crate::types::{Event, TransactionHash};
// use jsonrpsee::core::client::ClientT;
// use jsonrpsee::http_client::HttpClient;
// use jsonrpsee::rpc_params;
use sha3::{Digest, Keccak256};

pub fn compute_transaction_hash(events: &Vec<Event>) -> TransactionHash {
    let mut transaction_data_hasher = Keccak256::new();

    for event in events {
        match event {
            Event::CreateQuirkle { members, proof_ttl } => {
                for member in members {
                    transaction_data_hasher.update(member);
                }

                transaction_data_hasher.update(bytemuck::cast::<u64, [u8; 8]>(*proof_ttl));
            }
        }
    }

    let transaction_hash_vec = transaction_data_hasher.finalize();
    transaction_hash_vec.as_slice().try_into().unwrap()
}
