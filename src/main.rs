use std::env;
use jsonrpsee::core::async_trait;
use jsonrpsee::types::error::CALL_EXECUTION_FAILED_CODE;
use std::net::SocketAddr;
use std::ops::Add;
use std::sync::Arc;
use tokio::{select, time::{sleep_until, Duration, Instant}};
use types::{BlockHash, Transaction, TransactionHash, PendingTransactionRow, SurrealID, BlockRow};
use jsonrpsee::{server::Server, types::ErrorObjectOwned};
use sha3::{Digest, Keccak256};
use hex;
use alloy_primitives::{FixedBytes, B256};
use futures::prelude::stream::StreamExt;
use libp2p::{multiaddr, noise, ping, swarm::SwarmEvent, tcp, yamux};
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;
use surrealdb::engine::any;
use surrealdb::engine::any::Any as AnyDb;
use surrealdb::Surreal;
use surrealdb::error::Db as ErrorDb;
use tower_http::cors::{Any, CorsLayer};
use hyper::Method;

use quible_ecdsa_utils::{recover_signer_unchecked, sign_message};
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

async fn propose_block(block_number: u64, db_arc: &Arc<Surreal<AnyDb>>) {
    println!("new block! {}", block_number);

    let result = async {
        // Fetch pending transactions
        let pending_transaction_rows: Vec<PendingTransactionRow> = db_arc.select("pending_transactions").await?;

        let mut transaction_hashes: Vec<TransactionHash> = Vec::new();
        let mut transactions_json: Vec<serde_json::Value> = Vec::new();

        for row in &pending_transaction_rows {
            let hash_bytes_vec = hex::decode(row.hash.clone())?;
            let mut hash_bytes = [0u8; 32];
            hash_bytes[..32].copy_from_slice(&hash_bytes_vec[..32]);
            transaction_hashes.push(hash_bytes);
            transactions_json.push(serde_json::to_value(row.clone().data).unwrap());
        }

        let timestamp: u64 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| {
                ErrorDb::Thrown(format!("Failed to generate timestamp: {}", e).into())
            })?
            .as_secs();

        let block_hash = compute_block_hash(block_number, timestamp, transaction_hashes);

        // Insert new block
        /*
        db_arc.query("INSERT INTO blocks (hash, block_number, timestamp, transactions) VALUES ($hash, $block_number, $timestamp, $transactions)")
            .bind(("hash", block_hash))
            .bind(("block_number", block_number))
            .bind(("timestamp", timestamp))
            .bind(("transactions", transactions_json))
            .await?;
        */

        db_arc.create::<Vec<BlockRow>>("blocks")
            .content(BlockRow {
                hash: hex::encode(block_hash),
                block_number,
                timestamp,
                transactions: transactions_json
            })
            .await?;

        for row in pending_transaction_rows {
            let transaction = row.data;
            let hash = compute_transaction_hash(&transaction.events);
            let author = recover_signer_unchecked(&transaction.signature.bytes, &hash).unwrap();

            for event in transaction.events {
                match event {
                    types::Event::CreateQuirkle { members, proof_ttl, slug } => {
                        // Update quirkle count
                        let quirkle_count_result: Option<u64> = db_arc
                            .query("UPDATE author_quirkle_counts SET count += 1 WHERE author = $author RETURN count")
                            .bind(("author", hex::encode(author)))
                            .await?
                            .take::<Option<u64>>((0, "count"))?;

                        let quirkle_count = match quirkle_count_result {
                            Some(count) => count,
                            None => {
                                db_arc
                                    .query("INSERT INTO author_quirkle_counts (author, count) VALUES ($author, 1)")
                                    .bind(("author", hex::encode(author)))
                                    .await?;
                                1
                            }
                        };

                        let quirkle_root = compute_quirkle_root(author.into_array(), quirkle_count, slug);

                        // Insert quirkle proof TTL
                        db_arc.query("INSERT INTO quirkle_proof_ttls (quirkle_root, proof_ttl) VALUES ($quirkle_root, $proof_ttl)")
                            .bind(("quirkle_root", hex::encode(quirkle_root.bytes)))
                            .bind(("proof_ttl", proof_ttl))
                            .await?;

                        // Insert quirkle items
                        for member in members {
                            db_arc.query("INSERT INTO quirkle_items (quirkle_root, address) VALUES ($quirkle_root, string::lowercase($address))")
                                .bind(("quirkle_root", hex::encode(quirkle_root.bytes)))
                                .bind(("address", member))
                                .await?;
                        }
                    }
                }
            }

            db_arc.query("DELETE FROM pending_transactions WHERE id = $id")
                .bind(("id", row.id))
                .await?;
        }

        Ok(()) as Result<(), Box<dyn std::error::Error>>
    }.await;

    if let Err(e) = result {
        eprintln!("Error in propose_block: {:?}", e);
    }
}
pub struct QuibleRpcServerImpl {
    db: Arc<Surreal<AnyDb>>,
}

fn compute_quirkle_root(author: [u8; 20], contract_count: u64, slug: Option<String>) -> types::QuirkleRoot {
    let mut quirkle_data_hasher = Keccak256::new();
    quirkle_data_hasher.update(author);

    match slug {
        Some(text) => {
            quirkle_data_hasher.update(text);
        }

        None => {
            quirkle_data_hasher.update(bytemuck::cast::<u64, [u8; 8]>(contract_count));
        }
    }

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
impl quible_rpc::QuibleRpcServer for QuibleRpcServerImpl {
    async fn send_transaction(
        &self,
        transaction: Transaction,
    ) -> Result<Transaction, ErrorObjectOwned> {
        let transaction_hash = compute_transaction_hash(&transaction.events);
        // let transaction_json = serde_json::to_value(&transaction).unwrap();

        let transaction_hash_hex = hex::encode(transaction_hash);
        let result: Result<Vec<PendingTransactionRow>, surrealdb::Error> = self.db
            .create("pending_transactions")
            .content(PendingTransactionRow {
                id: SurrealID(Thing::from(("pending_transactions".to_string(), transaction_hash_hex.clone().to_string()))),
                // hash: surrealdb::sql::Bytes::from(transaction_hash.to_vec()),
                hash: transaction_hash_hex,
                data: transaction.clone()
            })
            .await;

        match result {
            Ok(pending_transaction_rows) => { 
                if pending_transaction_rows.len() == 0 {
                    Err(ErrorObjectOwned::owned::<String>(
                            CALL_EXECUTION_FAILED_CODE,
                            "call execution failed: transaction already inserted",
                            None
                    ))
                } else { 
                    Ok(transaction)
                }
            },
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
            .query("SELECT * FROM quirkle_items WHERE quirkle_root = $quirkle_root AND address = string::lowercase($address)")
            .bind(("quirkle_root", hex::encode(quirkle_root.bytes)))
            .bind(("address", &member_address))
            .await
            .and_then(|mut response| response.take(0));

        match result {
            Ok(Some(_)) => {
                let proof_ttl: Result<Option<u64>, surrealdb::Error> = self.db
                    .query("SELECT proof_ttl FROM quirkle_proof_ttls WHERE quirkle_root = $quirkle_root")
                    .bind(("quirkle_root", hex::encode(quirkle_root.bytes)))
                    .await
                    .and_then(|mut response| response.take((0, "proof_ttl")));

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

                let mut proof_data = Vec::<u8>::new();
                proof_data.extend(&quirkle_root.bytes);
                proof_data.extend(member_address.as_bytes());
                proof_data.extend(&expires_at.to_be_bytes());

                let mut proof_data_hasher = Keccak256::new();
                let prefix_str = format!("\x19Ethereum Signed Message:\n{}", proof_data.len());
                proof_data_hasher.update(prefix_str);
                proof_data_hasher.update(proof_data);

                let proof_hash_vec = proof_data_hasher.finalize();
                let proof_hash = proof_hash_vec.as_slice().try_into().unwrap();

                // TODO(QUI-19): pull in a real private key
                // TODO: construct the key before starting the server
                let signer_secret = k256::ecdsa::SigningKey::from_bytes(&hex_literal::hex!(
                    "ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
                ).into()).map_err(|e| {
                    ErrorObjectOwned::owned(
                        CALL_EXECUTION_FAILED_CODE,
                        "call execution failed: failed to construct signing key",
                        Some(e.to_string()),
                    )
                })?;

                let signature_bytes = sign_message(
                    B256::from_slice(&signer_secret.to_bytes()[..]),
                    FixedBytes::new(proof_hash),
                ).map_err(|e| {
                    ErrorObjectOwned::owned(
                        CALL_EXECUTION_FAILED_CODE,
                        "call execution failed: could not sign proof",
                        Some(e.to_string()),
                    )
                })?;

                Ok(types::QuirkleProof {
                    quirkle_root,
                    member_address,
                    expires_at,
                    signature: types::QuirkleSignature {
                        ecdsa_signature_bytes: signature_bytes
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

async fn run_derive_server(db: &Arc<Surreal<AnyDb>>, port: u16) -> anyhow::Result<SocketAddr> {
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

async fn initialize_db(db: &Surreal<AnyDb>) -> surrealdb::Result<()> {
    // Create table for blocks
    db.query("DEFINE TABLE blocks SCHEMAFULL;").await?;
    db.query("DEFINE FIELD hash ON blocks TYPE string;").await?;
    db.query("DEFINE FIELD block_number ON blocks TYPE int;").await?;
    // db.query("DEFINE FIELD timestamp ON blocks TYPE datetime;").await?;
    db.query("DEFINE FIELD timestamp ON blocks TYPE int;").await?;
    db.query("DEFINE FIELD transactions ON blocks TYPE array;").await?;
    db.query("DEFINE FIELD transactions.* ON blocks FLEXIBLE TYPE object;").await?;

    // Create table for pending transactions
    db.query("DEFINE TABLE pending_transactions SCHEMAFULL;").await?;
    // db.query("DEFINE FIELD hash ON pending_transactions TYPE bytes;").await?;
    db.query("DEFINE FIELD hash ON pending_transactions TYPE string;").await?;
    db.query("DEFINE FIELD data ON pending_transactions TYPE object;").await?;
    db.query("DEFINE FIELD data.signature ON pending_transactions TYPE string;").await?;
    db.query("DEFINE FIELD data.events ON pending_transactions TYPE array;").await?;

    // TODO: define the event type more thoroughly here to avoid the use of FLEXIBLE
    db.query("DEFINE FIELD data.events.* ON pending_transactions FLEXIBLE TYPE object;").await?;

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

// TODO: https://linear.app/quible/issue/QUI-49/refactor-entrypoint-for-easier-unit-testing
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let port: u16 = env::var("QUIBLE_PORT").unwrap_or_else(|_| "9013".to_owned()).parse()?;
    let endpoint = env::var("QUIBLE_DATABASE_URL").unwrap_or_else(|_| "memory".to_owned());
    let leader_addr = env::var("QUIBLE_LEADER_MULTIADDR").ok();
    // surrealdb init
    let db = any::connect(endpoint).await?;
    db.use_ns("quible").use_db("quible_node").await?;
    initialize_db(&db).await?;

    let db_arc = Arc::new(db);
    let server_addr = run_derive_server(&db_arc, port).await?;
    let url = format!("http://{}", server_addr);
    println!("server listening at {}", url);

    let mut block_number = 0u64;
    let mut block_timestamp = Instant::now();

    // TODO: derive identity from configurable ECDSA key
    let mut swarm = libp2p::SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_behaviour(|_| ping::Behaviour::default())?
        .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(u64::MAX)))
        .build();

    // TODO: https://linear.app/quible/issue/QUI-48/make-libp2p-port-configurable
    match leader_addr {
        None => {
            println!("listening as leader");
            swarm.listen_on("/ip4/0.0.0.0/tcp/9014".parse()?)?;
        }

        Some(_) => {
            swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;
        }
    };

    let remote_addr = leader_addr.clone().map(|url| {
        (url.clone(), url.parse::<multiaddr::Multiaddr>().unwrap())
    });

    match remote_addr.clone() {
        Some((url, addr)) => {
            match swarm.dial(addr) {
                Err(e) => {
                    eprintln!("Failed to dial {url}: {}", e);
                }

                _ => {}
            };

            println!("Dialed {url}");
        }

        _ => {}
    }

    propose_block(block_number, &db_arc).await;

    loop {
        select! {
            _ = sleep_until(block_timestamp + SLOT_DURATION) => {
                block_timestamp = block_timestamp + SLOT_DURATION;
                block_number += 1;

                propose_block(block_number, &db_arc).await;
            }

            event = swarm.select_next_some() => match event {
                SwarmEvent::NewListenAddr { address, .. } => println!("libp2p listening on {address:?}"),
                SwarmEvent::Behaviour(event) => println!("{event:?}"),

                // TODO(QUI-46): enable debug log level
                SwarmEvent::OutgoingConnectionError { .. } => println!("dial failure: {event:?}"),

                _ => {
                    // TODO(QUI-46): enable debug log level
                }
            }
        }
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

    #[tokio::test]
    async fn test_send_transaction() -> anyhow::Result<()> {
        // Initialize SurrealDB
        let db = any::connect("memory").await?;
        db.use_ns("quible").use_db("quible_node").await?;
        initialize_db(&db).await?;

        let db_arc = Arc::new(db);

        let server_addr = run_derive_server(&db_arc, 0).await?;
        let url = format!("http://{}", server_addr);
        println!("server listening at {}", url);
        let client = HttpClient::builder().build(url)?;
        let signer_secret = k256::ecdsa::SigningKey::random(&mut rand::thread_rng());
        let events = vec![types::Event::CreateQuirkle {
            members: vec![],
            proof_ttl: 86400,
            slug: None
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
        let pending_transaction_rows: Vec<PendingTransactionRow> = db_arc
            .select("pending_transactions")
            .await?;

        for row in pending_transaction_rows {
            println!("Transaction Hash: {}", row.hash);
            println!("Transaction Data: {}", serde_json::to_string_pretty(&row.data)?);
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_request_proof() -> anyhow::Result<()> {

        let db = any::connect("memory").await?;
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
            slug: None,
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
                compute_quirkle_root(author.into_array(), 1, None),
                "foo".to_string(),
                0,
            )
            .await
            .unwrap();
        dbg!("success response: {:?}", success_response);

        let failure_response = client
            .request_proof(
                compute_quirkle_root(author.into_array(), 1, None),
                "bar".to_string(),
                0,
            )
            .await;

        println!("failure response: {:?}", failure_response);

        Ok(())
    }
}
