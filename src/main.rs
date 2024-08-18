use std::sync::{Arc, Mutex};
use std::net::SocketAddr;
use jsonrpsee::types::error::CALL_EXECUTION_FAILED_CODE;
use jsonrpsee::core::async_trait;
use tokio::time::{Duration, Instant, sleep_until};
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
    db: Arc<Mutex<Connection>>
}

#[async_trait]
impl quible_rpc::QuibleRpcServer for QuibleRpcServerImpl {
    async fn send_transaction(&self, transaction: types::Transaction) -> Result<types::Transaction, ErrorObjectOwned> {
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
        db.execute("
            INSERT INTO pending_transactions (hash, data)
            VALUES (?1, ?2)
        ", (transaction_hash, transaction_json)).map_err(|error| {
            ErrorObjectOwned::owned(
                CALL_EXECUTION_FAILED_CODE,
                "call execution failed: failed to insert",
                Some(error.to_string())
            )
        })?;
        Ok(transaction)
    }
}

async fn run_derive_server(conn: Connection) -> anyhow::Result<SocketAddr> {
    let server = Server::builder().build("127.0.0.1:9013".parse::<SocketAddr>()?).await?;

    let addr = server.local_addr()?;
    let handle = server.start(QuibleRpcServerImpl { db: Arc::new(Mutex::new(conn)) }.into_rpc());

    tokio::spawn(handle.stopped());

    Ok(addr)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let conn = Connection::open_in_memory()?;
    conn.execute("
        CREATE TABLE pending_transactions (
          hash STRING PRIMARY KEY,
          data JSON
        )
    ", ())?;

    let server_addr = run_derive_server(conn).await?;
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
