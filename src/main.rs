use jsonrpsee::core::async_trait;
use jsonrpsee::types::error::CALL_EXECUTION_FAILED_CODE;
use std::net::SocketAddr;
use std::ops::Add;
use std::sync::Arc;
use tokio::time::{sleep_until, Duration, Instant};
use types::{BlockHash, Transaction, TransactionHash};
use jsonrpsee::{server::Server, types::ErrorObjectOwned};
use sha3::{Digest, Keccak256};
use hex;

use serde::{Deserialize, Serialize};
use surrealdb::engine::local::Mem;
use surrealdb::sql::Thing;
use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use surrealdb::error::Db as ErrorDb;
use tower_http::cors::{Any, CorsLayer};
use hyper::Method;

use quible_ecdsa_utils::recover_signer_unchecked;
use quible_rpc::QuibleRpcServer;
use quible_transaction_utils::compute_transaction_hash;

pub mod quible_ecdsa_utils;
pub mod quible_rpc;
pub mod quible_transaction_utils;
pub mod types;

const SLOT_DURATION: Duration = Duration::from_secs(4);

#[derive(Debug, Deserialize, Serialize)]
struct Record {
    #[allow(dead_code)]
    id: Thing,
}

async fn propose_block(block_number: u64, db_arc: &Arc<Surreal<Db>>) {
    println!("new block! {}", block_number);

    let result = async {
        // Fetch pending transactions
        let transactions: Vec<(TransactionHash, Transaction)> = db_arc
            .query("SELECT hash, data FROM pending_transactions")
            .await?
            .take(0)?;

        let mut transaction_hashes: Vec<TransactionHash> = Vec::new();
        let mut transactions_json: Vec<serde_json::Value> = Vec::new();

        for (hash, transaction) in &transactions {
            transaction_hashes.push(*hash);
            transactions_json.push(serde_json::to_value(transaction).unwrap());
        }

        let timestamp: u64 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| {
                ErrorDb::Thrown(format!("Failed to generate timestamp: {}", e).into())
            })?
            .as_secs();

        let block_hash = compute_block_hash(block_number, timestamp, transaction_hashes);

        // Insert new block
        db_arc.query("INSERT INTO blocks (hash, block_number, timestamp, transactions) VALUES ($hash, $block_number, $timestamp, $transactions)")
            .bind(("hash", block_hash))
            .bind(("block_number", block_number))
            .bind(("timestamp", timestamp))
            .bind(("transactions", serde_json::Value::Array(transactions_json)))
            .await?;

        for (_, transaction) in transactions {
            let hash = compute_transaction_hash(&transaction.events);
            let author = recover_signer_unchecked(&transaction.signature.bytes, &hash).unwrap();

            for event in transaction.events {
                match event {
                    types::Event::CreateQuirkle { members, proof_ttl } => {
                        // Update quirkle count
                        let quirkle_count: u64 = db_arc
                            .query("UPDATE author_quirkle_counts SET count += 1 WHERE author = $author RETURN count")
                            .bind(("author", author.into_array()))
                            .await?
                            .take::<Option<u64>>(0)?
                            .unwrap_or(1); 

                        let quirkle_root = compute_quirkle_root(author.into_array(), quirkle_count);

                        // Insert quirkle proof TTL
                        db_arc.query("INSERT INTO quirkle_proof_ttls (quirkle_root, proof_ttl) VALUES ($quirkle_root, $proof_ttl)")
                            .bind(("quirkle_root", quirkle_root.bytes))
                            .bind(("proof_ttl", proof_ttl))
                            .await?;

                        // Insert quirkle items
                        for member in members {
                            db_arc.query("INSERT INTO quirkle_items (quirkle_root, address) VALUES ($quirkle_root, $address)")
                                .bind(("quirkle_root", quirkle_root.bytes))
                                .bind(("address", member))
                                .await?;
                        }
                    }
                }
            }
        }

        Ok(()) as Result<(), Box<dyn std::error::Error>>
    }.await;

    if let Err(e) = result {
        eprintln!("Error in propose_block: {:?}", e);
    }
}
pub struct QuibleRpcServerImpl {
    db: Arc<Surreal<Db>>,
}

fn compute_quirkle_root(author: [u8; 20], contract_count: u64) -> types::QuirkleRoot {
    let mut quirkle_data_hasher = Keccak256::new();
    quirkle_data_hasher.update(author);
    quirkle_data_hasher.update(bytemuck::cast::<u64, [u8; 8]>(contract_count));

    let quirkle_hash_vec = quirkle_data_hasher.finalize();
    types::QuirkleRoot {
        bytes: quirkle_hash_vec.as_slice().try_into().unwrap(),
    }
}

// TODO(QUI-17): refactor this such that there is only a block_header parameter
//               instead of individual parameters
fn compute_block_hash(
    block_number: u64,
    timestamp: u64,
    transaction_hashes: Vec<TransactionHash>,
) -> BlockHash {
    let mut block_data_hasher = Keccak256::new();

    block_data_hasher.update(bytemuck::cast::<u64, [u8; 8]>(block_number));
    block_data_hasher.update(bytemuck::cast::<u64, [u8; 8]>(timestamp));

    for transaction_hash in transaction_hashes {
        block_data_hasher.update(transaction_hash);
    }

    let block_hash_vec = block_data_hasher.finalize();
    block_hash_vec.as_slice().try_into().unwrap()
}

#[async_trait]
#[async_trait]
impl quible_rpc::QuibleRpcServer for QuibleRpcServerImpl {
    async fn send_transaction(
        &self,
        transaction: Transaction,
    ) -> Result<Transaction, ErrorObjectOwned> {
        let transaction_hash = compute_transaction_hash(&transaction.events);
        let transaction_json = serde_json::to_string(&transaction).unwrap();

        let transaction_hash_hex = hex::encode(transaction_hash);
        let result: Result<Option<Thing>, surrealdb::Error> = self.db
            .create(("pending_transactions", transaction_hash_hex))
            .content(serde_json::json!({
                "hash": transaction_hash,
                "data": transaction_json,
            }))
            .await;

        match result {
            Ok(Some(_)) => Ok(transaction),
            Ok(None) => Err(ErrorObjectOwned::owned::<String>(
                CALL_EXECUTION_FAILED_CODE,
                "call execution failed: record not created",
                None,
            )),
            Err(error) => Err(ErrorObjectOwned::owned::<String>(
                CALL_EXECUTION_FAILED_CODE,
                "call execution failed: failed to insert",
                Some(error.to_string()),
            )),
        }
    }
    async fn request_proof(
        &self,
        quirkle_root: types::QuirkleRoot,
        member_address: String,
        _requested_at_block_number: u128,
    ) -> Result<types::QuirkleProof, ErrorObjectOwned> {
        let result: Result<Option<serde_json::Value>, surrealdb::Error> = self.db
            .query("SELECT * FROM quirkle_items WHERE quirkle_root = $quirkle_root AND address = $address")
            .bind(("quirkle_root", quirkle_root.bytes))
            .bind(("address", &member_address))
            .await
            .and_then(|mut response| response.take(0));

        match result {
            Ok(Some(_)) => {
                let proof_ttl: Result<Option<u64>, surrealdb::Error> = self.db
                    .query("SELECT proof_ttl FROM quirkle_proof_ttls WHERE quirkle_root = $quirkle_root")
                    .bind(("quirkle_root", quirkle_root.bytes))
                    .await
                    .and_then(|mut response| response.take(0));

                let proof_ttl = proof_ttl.map_err(|e| ErrorObjectOwned::owned(
                    CALL_EXECUTION_FAILED_CODE,
                    "call execution failed: failed to query proof_ttl",
                    Some(e.to_string()),
                ))?.unwrap_or(3600); // Default to 1 hour if not found

                let expires_at: u64 = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_err(|e| {
                        ErrorObjectOwned::owned(
                            CALL_EXECUTION_FAILED_CODE,
                            "call execution failed: could not generate timestamp",
                            Some(e.to_string()),
                        )
                    })?
                    .add(Duration::from_secs(proof_ttl))
                    .as_secs();

                let mut content = Vec::new();
                content.extend_from_slice(&quirkle_root.bytes);
                content.extend_from_slice(member_address.as_bytes());
                content.extend_from_slice(&expires_at.to_le_bytes());

                Ok(types::QuirkleProof {
                    quirkle_root,
                    member_address,
                    expires_at,
                    signature: types::QuirkleSignature {
                        // TODO(QUI-19): pull in a real private key
                        bls_signature: bls_signatures::PrivateKey::new([
                            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
                            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
                            0x0, 0x0, 0x0, 0x0,
                        ])
                        .sign(content),
                    },
                })
            }
            Ok(None) => Err(ErrorObjectOwned::owned(
                CALL_EXECUTION_FAILED_CODE,
                "call execution failed: could not verify membership",
                Some("address not found in the quirkle"),
            )),
            Err(e) => Err(ErrorObjectOwned::owned(
                CALL_EXECUTION_FAILED_CODE,
                "call execution failed: database query error",
                Some(e.to_string()),
            )),
        }
    }
}

async fn run_derive_server(db: &Arc<Surreal<Db>>, port: u16) -> anyhow::Result<SocketAddr> {
    let cors = CorsLayer::new()
		// Allow `POST` when accessing the resource
		.allow_methods([Method::POST])
		// Allow requests from any origin
		.allow_origin(Any)
		.allow_headers([hyper::header::CONTENT_TYPE]);
    let middleware = tower::ServiceBuilder::new().layer(cors);

    let server = Server::builder()
        .set_http_middleware(middleware)
        .build(format!("127.0.0.1:{}", port).parse::<SocketAddr>()?)
        .await?;



    let addr = server.local_addr()?;
    let handle = server.start(QuibleRpcServerImpl { db: db.clone() }.into_rpc());

    tokio::spawn(handle.stopped());

    Ok(addr)
}

async fn initialize_db(db: &Surreal<Db>) -> surrealdb::Result<()> {
    // Create table for blocks
    db.query("DEFINE TABLE blocks SCHEMAFULL;").await?;
    db.query("DEFINE FIELD hash ON blocks TYPE string;").await?;
    db.query("DEFINE FIELD block_number ON blocks TYPE int;").await?;
    db.query("DEFINE FIELD timestamp ON blocks TYPE datetime;").await?;
    db.query("DEFINE FIELD transactions ON blocks TYPE array;").await?;

    // Create table for pending transactions
    db.query("DEFINE TABLE pending_transactions SCHEMAFULL;").await?;
    db.query("DEFINE FIELD hash ON pending_transactions TYPE string;").await?;
    db.query("DEFINE FIELD data ON pending_transactions TYPE object;").await?;

    // Create table for author quirkle counts
    db.query("DEFINE TABLE author_quirkle_counts SCHEMAFULL;").await?;
    db.query("DEFINE FIELD author ON author_quirkle_counts TYPE string;").await?;
    db.query("DEFINE FIELD count ON author_quirkle_counts TYPE int;").await?;

    // Create table for quirkle proof TTLs
    db.query("DEFINE TABLE quirkle_proof_ttls SCHEMAFULL;").await?;
    db.query("DEFINE FIELD quirkle_root ON quirkle_proof_ttls TYPE string;").await?;
    db.query("DEFINE FIELD proof_ttl ON quirkle_proof_ttls TYPE int;").await?;

    // Create table for quirkle items
    db.query("DEFINE TABLE quirkle_items SCHEMAFULL;").await?;
    db.query("DEFINE FIELD quirkle_root ON quirkle_items TYPE string;").await?;
    db.query("DEFINE FIELD address ON quirkle_items TYPE string;").await?;

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {

    // surrealdb init
    let db = Surreal::new::<Mem>(()).await?;
    db.use_ns("quible").use_db("quible_node").await?;
    initialize_db(&db).await?;

    let db_arc = Arc::new(db);
    let server_addr = run_derive_server(&db_arc, 9013).await?;
    let url = format!("http://{}", server_addr);
    println!("server listening at {}", url);

    let mut block_number = 0u64;
    let mut block_timestamp = Instant::now();

    loop {
        propose_block(block_number, &db_arc).await;

        sleep_until(block_timestamp + SLOT_DURATION).await;

        block_timestamp = block_timestamp + SLOT_DURATION;
        block_number += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quible_rpc::QuibleRpcClient;
    use alloy_primitives::{FixedBytes, B256};
    use jsonrpsee::http_client::HttpClient;
    use quible_ecdsa_utils::{recover_signer_unchecked, sign_message};
    use types::ECDSASignature;
    use surrealdb::Surreal;
    use surrealdb::engine::local::Mem;

    #[tokio::test]
    async fn test_send_transaction() -> anyhow::Result<()> {
        // Initialize SurrealDB
        let db = Surreal::new::<Mem>(()).await?;
        db.use_ns("quible").use_db("quible_node").await?;
        initialize_db(&db).await?;

        let db_arc = Arc::new(db);

        let server_addr = run_derive_server(&db_arc, 9013).await?;
        let url = format!("http://{}", server_addr);
        println!("server listening at {}", url);
        let client = HttpClient::builder().build(url)?;
        let signer_secret = k256::ecdsa::SigningKey::random(&mut rand::thread_rng());
        let events = vec![types::Event::CreateQuirkle {
            members: vec![],
            proof_ttl: 86400,
        }];
        let hash = compute_transaction_hash(&events);
        let signature_bytes = sign_message(
            B256::from_slice(&signer_secret.to_bytes()[..]),
            FixedBytes::new(hash),
        )?;
        let signature = ECDSASignature {
            bytes: signature_bytes,
        };
        let transaction = Transaction { signature, events };

        // let params = rpc_params![transaction];
        // let params = rpc_params![json!({"events": [{"name": "CreateQuirkle", "members": [], "proof_ttl": 86400}]})];
        // let response: Result<String, _> = client.request("quible_sendTransaction", params).await;
        let response = client.send_transaction(transaction).await.unwrap();
        dbg!("response: {:?}", response);

        // Query pending transactions from SurrealDB
        let pending_transactions: Vec<(String, serde_json::Value)> = db_arc
            .query("SELECT hash, data FROM pending_transactions")
            .await?
            .take(0)?;

        for (hash, data) in pending_transactions {
            println!("Transaction Hash: {}", hash);
            println!("Transaction Data: {}", serde_json::to_string_pretty(&data)?);
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_request_proof() -> anyhow::Result<()> {

        let db = Surreal::new::<Mem>(()).await?;
        db.use_ns("quible").use_db("quible_node").await?;
        initialize_db(&db).await?;

        let db_arc = Arc::new(db);

        let server_addr = run_derive_server(&db_arc, 0).await?;
        let url = format!("http://{}", server_addr);
        println!("server listening at {}", url);
        let client = HttpClient::builder().build(url)?;

        let signer_secret = k256::ecdsa::SigningKey::random(&mut rand::thread_rng());
        let events = vec![types::Event::CreateQuirkle {
            members: vec!["foo".to_string()],
            proof_ttl: 86400,
        }];
        let hash = compute_transaction_hash(&events);
        let signature_bytes = sign_message(
            B256::from_slice(&signer_secret.to_bytes()[..]),
            FixedBytes::new(hash),
        )?;
        let signature = ECDSASignature {
            bytes: signature_bytes,
        };
        let author = recover_signer_unchecked(&signature_bytes, &hash)?;
        let transaction = Transaction { signature, events };

        client.send_transaction(transaction).await.unwrap();

        propose_block(1, &db_arc).await;

        let success_response = client
            .request_proof(
                compute_quirkle_root(author.into_array(), 1),
                "foo".to_string(),
                0,
            )
            .await
            .unwrap();
        dbg!("success response: {:?}", success_response);

        let failure_response = client
            .request_proof(
                compute_quirkle_root(author.into_array(), 1),
                "bar".to_string(),
                0,
            )
            .await;

        println!("failure response: {:?}", failure_response);

        Ok(())
    }
}
