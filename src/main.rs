use jsonrpsee::core::async_trait;
use jsonrpsee::types::error::CALL_EXECUTION_FAILED_CODE;
use std::net::SocketAddr;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::{sleep_until, Duration, Instant};
use types::{BlockHash, Transaction, TransactionHash};
use jsonrpsee::{server::Server, types::ErrorObjectOwned};
use rusqlite::{Connection, Result};
use sha3::{Digest, Keccak256};

use quible_ecdsa_utils::recover_signer_unchecked;
use quible_rpc::QuibleRpcServer;
use quible_transaction_utils::compute_transaction_hash;

pub mod quible_ecdsa_utils;
pub mod quible_rpc;
pub mod quible_transaction_utils;
pub mod types;

const SLOT_DURATION: Duration = Duration::from_secs(4);

async fn propose_block(block_number: u64, conn_arc: &Arc<Mutex<Connection>>) {
    println!("new block! {}", block_number);

    let conn_lock = conn_arc.lock().unwrap();
    let mut stmt = conn_lock
        .prepare("SELECT hash, data FROM pending_transactions")
        .unwrap();
    let transactions_iter_result: Result<Vec<([u8; 32], Transaction)>, rusqlite::Error> = stmt
        .query_map([], |row| {
            let raw_hash = row.get::<_, [u8; 32]>(0)?;
            let data = row.get::<_, serde_json::value::Value>(1)?;
            let transaction: Transaction = serde_json::from_value(data).unwrap();
            Ok((raw_hash, transaction))
        })
        .unwrap()
        .collect();

    let transactions_iter = transactions_iter_result.unwrap();

    let mut transaction_hashes: Vec<TransactionHash> = Vec::new();
    let mut transactions: Vec<Transaction> = Vec::new();
    let mut transactions_json: Vec<serde_json::value::Value> = Vec::new();

    for transaction in transactions_iter.iter() {
        let (hash, transaction) = transaction;
        transaction_hashes.push(*hash);
        transactions_json.push(serde_json::to_value(transaction).unwrap());
        transactions.push(transaction.clone());
    }

    let timestamp: u64 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| {
            ErrorObjectOwned::owned(
                CALL_EXECUTION_FAILED_CODE,
                "call execution failed: could not generate timestamp",
                Some(e.to_string()),
            )
        })
        .unwrap()
        .as_secs();

    let block_hash = compute_block_hash(block_number, timestamp, transaction_hashes);

    let params = (
        block_hash,
        block_number,
        timestamp,
        serde_json::value::Value::Array(transactions_json),
    );

    // TODO(QUI-15): generate merkle root of transactions
    // TODO(QUI-17): refactor to use unified blocker header type

    conn_lock
        .execute(
            "
            INSERT INTO blocks (hash, block_number, timestamp, transactions)
            VALUES (?1, ?2, ?3, ?4)
            ",
            params,
        )
        .unwrap();

    for transaction in transactions {
        let hash = compute_transaction_hash(&transaction.events);
        let author = recover_signer_unchecked(&transaction.signature.bytes, &hash).unwrap();

        for event in transaction.events {
            match event {
                types::Event::CreateQuirkle { members, proof_ttl } => {
                    // TODO(QUI-18): replace usage of quirkle counts by utilizing UTXO addresses instead
                    let quirkle_count: u64 = conn_lock
                        .query_row(
                            "
                            INSERT INTO author_quirkle_counts (author, count)
                            VALUES (?1, 1)
                            ON CONFLICT (author) DO UPDATE SET
                              count = count + 1
                            RETURNING count
                            ",
                            [author.into_array()],
                            |row| row.get(0),
                        )
                        .unwrap();

                    let quirkle_root = compute_quirkle_root(author.into_array(), quirkle_count);

                    conn_lock
                        .execute(
                            "
                            INSERT INTO quirkle_proof_ttls (quirkle_root, proof_ttl)
                            VALUES (?1, ?2)
                            ",
                            (quirkle_root.bytes, proof_ttl),
                        )
                        .unwrap();

                    for member in members {
                        conn_lock
                            .execute(
                                "
                                INSERT INTO quirkle_items (quirkle_root, address)
                                VALUES (?1, ?2)
                                ",
                                (quirkle_root.bytes, member),
                            )
                            .unwrap();
                    }
                }
            }
        }
    }
}

pub struct QuibleRpcServerImpl {
    db: Arc<Mutex<Connection>>,
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
impl quible_rpc::QuibleRpcServer for QuibleRpcServerImpl {
    async fn send_transaction(
        &self,
        transaction: Transaction,
    ) -> Result<Transaction, ErrorObjectOwned> {
        let transaction_hash = compute_transaction_hash(&transaction.events);
        let transaction_json = serde_json::to_string(&transaction).unwrap();
        let db = &self.db.lock().unwrap();

        db.execute(
            "
            INSERT INTO pending_transactions (hash, data)
            VALUES (?1, ?2)
            ",
            (transaction_hash, transaction_json),
        )
        .map_err(|error| {
            ErrorObjectOwned::owned(
                CALL_EXECUTION_FAILED_CODE,
                "call execution failed: failed to insert",
                Some(error.to_string()),
            )
        })?;

        Ok(transaction)
    }

    async fn request_proof(
        &self,
        quirkle_root: types::QuirkleRoot,
        member_address: String,
        _requested_at_block_number: u128,
    ) -> Result<types::QuirkleProof, ErrorObjectOwned> {
        let db = &self.db.lock().unwrap();

        let result: Result<(), rusqlite::Error> = db.query_row(
            "
            SELECT quirkle_root, address FROM quirkle_items
            WHERE quirkle_root = ?1 AND address = ?2
            ",
            (quirkle_root.bytes, &member_address),
            |_| Ok(()),
        );

        match result {
            Ok(()) => {
                let proof_ttl: u64 = db
                    .query_row(
                        "
                        SELECT proof_ttl FROM quirkle_proof_ttls
                        WHERE quirkle_root = ?1
                        ",
                        [quirkle_root.bytes],
                        |row| row.get(0),
                    )
                    .unwrap();

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

            Err(rusqlite::Error::QueryReturnedNoRows) => Err(ErrorObjectOwned::owned(
                CALL_EXECUTION_FAILED_CODE,
                "call execution failed: could not verify membership",
                Some("address not found in the quirkle"),
            )),

            Err(e) => Err(ErrorObjectOwned::owned(
                CALL_EXECUTION_FAILED_CODE,
                "call execution failed: failed to query",
                Some(e.to_string()),
            )),
        }
    }
}

async fn run_derive_server(conn: &Arc<Mutex<Connection>>, port: u16) -> anyhow::Result<SocketAddr> {
    let server = Server::builder()
        .build(format!("127.0.0.1:{}", port).parse::<SocketAddr>()?)
        .await?;

    let addr = server.local_addr()?;
    // let handle = server.start(QuibleRpcServerImpl { db: Arc::new(Mutex::new(conn)) }.into_rpc());
    let handle = server.start(QuibleRpcServerImpl { db: conn.clone() }.into_rpc());

    tokio::spawn(handle.stopped());

    Ok(addr)
}

fn initialize_db(conn: &Connection) -> anyhow::Result<()> {
    conn.execute(
        "
        CREATE TABLE pending_transactions (
          hash BLOB PRIMARY KEY,
          data JSON
        )
        ",
        (),
    )?;

    conn.execute(
        // TODO(QUI-15): include transactions root blob
        // TODO(QUI-17): refactor to have entire block_header
        "
        CREATE TABLE blocks (
          hash BLOB PRIMARY KEY,
          block_number INT,
          timestamp DATETIME,
          transactions JSON
        )
        ",
        (),
    )?;

    conn.execute(
        "
        CREATE TABLE author_quirkle_counts (
          author BLOB PRIMARY KEY,
          count INT
        )
        ",
        (),
    )?;

    conn.execute(
        "
        CREATE TABLE quirkle_proof_ttls (
          quirkle_root BLOB PRIMARY KEY,
          proof_ttl INT
        )
        ",
        (),
    )?;

    conn.execute(
        "
        CREATE TABLE quirkle_items (
          quirkle_root BLOB,
          address TEXT,
          PRIMARY KEY (quirkle_root, address)
        )
        ",
        (),
    )?;

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let conn = Connection::open_in_memory()?;
    initialize_db(&conn)?;

    let conn_arc = Arc::new(Mutex::new(conn));

    let server_addr = run_derive_server(&conn_arc, 9013).await?;
    let url = format!("http://{}", server_addr);
    println!("server listening at {}", url);

    let mut block_number = 0u64;
    let mut block_timestamp = Instant::now();

    loop {
        propose_block(block_number, &conn_arc).await;

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

    #[tokio::test]
    async fn test_send_transaction() -> anyhow::Result<()> {
        let conn = Connection::open_in_memory()?;
        initialize_db(&conn)?;

        let conn_arc = Arc::new(Mutex::new(conn));

        let server_addr = run_derive_server(&conn_arc, 9013).await?;
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

        let conn_lock = conn_arc.lock().unwrap();
        let mut stmt = conn_lock.prepare("SELECT hash, data FROM pending_transactions")?;
        let row_iter = stmt.query_map([], |row| {
            let raw_hash = row.get::<_, [u8; 32]>(0)?;
            let hash = format!(
                "0x{}",
                raw_hash
                    .iter()
                    .map(|byte| format!("{:02x}", byte))
                    .collect::<String>()
            );
            let data = row.get::<_, serde_json::value::Value>(1)?;
            Ok((hash, data))
        })?;

        for row in row_iter {
            println!("Found row {:?}", row.unwrap());
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_request_proof() -> anyhow::Result<()> {
        let conn = Connection::open_in_memory()?;
        initialize_db(&conn)?;

        let conn_arc = Arc::new(Mutex::new(conn));

        let server_addr = run_derive_server(&conn_arc, 0).await?;
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

        propose_block(1, &conn_arc).await;

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
