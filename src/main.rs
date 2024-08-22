use jsonrpsee::core::async_trait;
use jsonrpsee::types::error::CALL_EXECUTION_FAILED_CODE;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::time::{sleep_until, Duration, Instant};
// use jsonrpsee::core::client::ClientT;
// use jsonrpsee::http_client::HttpClient;
// use jsonrpsee::rpc_params;
use jsonrpsee::{server::Server, types::ErrorObjectOwned};
use rusqlite::{Connection, Result};
use sha3::{Digest, Keccak256};

use quible_rpc::QuibleRpcServer;

pub mod quible_rpc;
pub mod types;

const SLOT_DURATION: Duration = Duration::from_secs(4);

fn propose_block(block_number: i64) {
    println!("new block! {}", block_number);

    // TODO(surrealdb): query transaction pool
    // TODO: generate merkle root of transactions
    // TODO: format block header with block number, timestamp, merkle root
    // TODO(gossip): broadcast block header and transaction list
    // TODO(surrealdb): insert blocks into db
}

pub struct QuibleRpcServerImpl {
    db: Arc<Mutex<Connection>>,
}

#[async_trait]
impl quible_rpc::QuibleRpcServer for QuibleRpcServerImpl {
    async fn send_transaction(
        &self,
        transaction: types::Transaction,
    ) -> Result<types::Transaction, ErrorObjectOwned> {
        let mut transaction_data_hasher = Keccak256::new();
        for event in transaction.clone().events {
            match event {
                types::Event::CreateQuirkle { members, proof_ttl } => {
                    for member in members {
                        transaction_data_hasher.update(member);
                    }
                    transaction_data_hasher.update(bytemuck::cast::<u64, [u8; 8]>(proof_ttl));
                }
            }
        }
        let transaction_hash: [u8; 32]; // TODO: check that this is the right size
        let transaction_hash_vec = transaction_data_hasher.finalize();
        transaction_hash = transaction_hash_vec.as_slice().try_into().unwrap();
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
        quirkle_root: String,
        member_address: String,
        _requested_at_block_number: u128,
    ) -> Result<types::QuirkleProof, ErrorObjectOwned> {
        let expires_at: u64 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| {
                ErrorObjectOwned::owned(
                    CALL_EXECUTION_FAILED_CODE,
                    "call execution failed: could not generate timestamp",
                    Some(e.to_string()),
                )
            })?
            .as_secs();

        let mut content = Vec::new();

        content.extend_from_slice(quirkle_root.as_bytes());
        content.extend_from_slice(member_address.as_bytes());
        content.extend_from_slice(&expires_at.to_le_bytes());

        Ok(types::QuirkleProof {
            quirkle_root,
            member_address,
            expires_at,
            signature: types::QuirkleSignature {
                // TODO: pull in a real private key
                bls_signature: bls_signatures::PrivateKey::new([
                    0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
                    0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
                ])
                .sign(content),
            },
        })
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

    // TODO: move proposal loop into a thread or something async
    // TODO: handle an incoming transaction over RPC
    // TODO(surrealdb): start a transaction pool table
    // TODO(surrealdb): insert incoming transactions into transaction pool
    //
    // TODO(surrealdb):
    //   setup some kind of testing script that sends transactions and verifies
    //   that the correct blocks are seen in the db

    let mut block_number = 0i64;
    let mut block_timestamp = Instant::now();

    loop {
        propose_block(block_number);

        sleep_until(block_timestamp + SLOT_DURATION).await;

        block_timestamp = block_timestamp + SLOT_DURATION;
        block_number += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quible_rpc::QuibleRpcClient;
    use jsonrpsee::http_client::HttpClient;

    #[tokio::test]
    async fn test_send_transaction() -> anyhow::Result<()> {
        let conn = Connection::open_in_memory()?;
        initialize_db(&conn)?;

        let conn_arc = Arc::new(Mutex::new(conn));

        let server_addr = run_derive_server(&conn_arc, 9013).await?;
        let url = format!("http://{}", server_addr);
        println!("server listening at {}", url);
        let client = HttpClient::builder().build(url)?;
        let transaction = types::Transaction {
            events: vec![types::Event::CreateQuirkle {
                members: vec![],
                proof_ttl: 86400,
            }],
        };

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

        // let params = rpc_params![transaction];
        // let params = rpc_params![json!({"events": [{"name": "CreateQuirkle", "members": [], "proof_ttl": 86400}]})];
        // let response: Result<String, _> = client.request("quible_sendTransaction", params).await;
        let response = client.request_proof("foo".to_string(), "bar".to_string(), 0).await.unwrap();
        dbg!("response: {:?}", response);

        Ok(())
    }
}
