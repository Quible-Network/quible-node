use std::net::SocketAddr;
use jsonrpsee::{core::async_trait, proc_macros::rpc};
use tokio::time::{Duration, Instant, sleep_until};
// use jsonrpsee::core::client::ClientT;
// use jsonrpsee::http_client::HttpClient;
// use jsonrpsee::rpc_params;
use jsonrpsee::{server::{RpcModule, Server}, types::ErrorObjectOwned};

mod types;

const SLOT_DURATION: Duration = Duration::from_secs(4);

fn propose_block(block_number: i64) {
    println!("new block! {}", block_number);

    // TODO(surrealdb): query transaction pool
    // TODO: generate merkle root of transactions
    // TODO: format block header with block number, timestamp, merkle root
    // TODO(gossip): broadcast block header and transaction list
    // TODO(surrealdb): insert blocks into db
}

#[rpc(server, client, namespace = "quible")]
pub trait RpcServer {
    #[method(name = "sendTransaction")]
    async fn send_transaction(&self, transaction: types::Transaction) -> Result<types::Transaction, ErrorObjectOwned>;
}

pub struct RpcServerImpl;

#[async_trait]
impl RpcServer for RpcServerImpl {
    async fn send_transaction(&self, transaction: types::Transaction) -> Result<types::Transaction, ErrorObjectOwned> {
        Ok(transaction)
    }
}

async fn run_server() -> anyhow::Result<SocketAddr> {
    // TODO: make port configurable
    let server = Server::builder().build("127.0.0.1:9013".parse::<SocketAddr>()?).await?;
    let mut module = RpcModule::new(());
    module.register_method::<Result<types::Transaction, ErrorObjectOwned>, _>("quible_sendTransaction", |params, _, _| {
        let params: types::Transaction = params.parse()?;
        Ok(params)
    })?;

    let addr = server.local_addr()?;
    let handle = server.start(module);

    tokio::spawn(handle.stopped());

    Ok(addr)
}

async fn run_derive_server() -> anyhow::Result<SocketAddr> {
    let server = Server::builder().build("127.0.0.1:9013".parse::<SocketAddr>()?).await?;

    let addr = server.local_addr()?;
    let handle = server.start(RpcServerImpl.into_rpc());

    tokio::spawn(handle.stopped());

    Ok(addr)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server_addr = run_derive_server().await?;
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
