use crate::types::{Event, TransactionHash};
// use jsonrpsee::core::client::ClientT;
// use jsonrpsee::http_client::HttpClient;
// use jsonrpsee::rpc_params;
use sha3::{Digest, Keccak256};

pub fn compute_transaction_hash(events: &Vec<Event>) -> TransactionHash {
    let mut data = Vec::<u8>::new();

    for event in events {
        match event {
            Event::CreateQuirkle {
                members,
                proof_ttl,
                slug,
            } => {
                for member in members {
                    data.extend(member.clone().into_bytes());
                }

                // TODO(QUI-35): including proof_ttl in tx hash
                // transaction_data_hasher.update(bytemuck::cast::<u64, [u8; 8]>(*proof_ttl));

                /*
                // TODO(QUI-35): including slug in tx hash
                match slug {
                    Some(text) => {
                        transaction_data_hasher.update(text);
                    }
                    _ => {}
                }
                */
            }
        }
    }

    let mut transaction_data_hasher = Keccak256::new();
    let prefix_str = format!("\x19Ethereum Signed Message:\n{}", data.len());
    transaction_data_hasher.update(prefix_str);
    transaction_data_hasher.update(data);
    let transaction_hash_vec = transaction_data_hasher.finalize();
    transaction_hash_vec.as_slice().try_into().unwrap()
}
