use alloy_primitives::{FixedBytes, B256};
use anyhow::anyhow;
use async_trait::async_trait;
use cert::types::{CertificateSigningRequestDetails, QuibleSignature, SignedCertificate};
use db::types::{
    BlockRow, ObjectRow, PendingTransactionRow, SurrealID, TrackerPing, TransactionOutputRow,
};
use futures::prelude::stream::StreamExt;
use hex;
use hyper::Method;
use jsonrpsee::core::async_trait as jsonrpsee_async_trait;
use jsonrpsee::types::error::CALL_EXECUTION_FAILED_CODE;
use jsonrpsee::{server::Server, types::ErrorObjectOwned};
use libp2p::{multiaddr, noise, ping, swarm::SwarmEvent, tcp, yamux};
use quible_ecdsa_utils::sign_message;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::net::SocketAddr;
use std::sync::Arc;
use surrealdb::engine::any;
use surrealdb::engine::any::Any as AnyDb;
use surrealdb::error::Db as ErrorDb;
use surrealdb::sql::Thing;
use surrealdb::Surreal;
use tokio::{
    select,
    time::{sleep_until, Duration, Instant},
};
use tower_http::cors::{Any, CorsLayer};
use tx::engine::{collect_valid_block_transactions, ExecutionContext};
use tx::types::{
    BlockHeader, Hashable, ObjectIdentifier, ObjectMode, Transaction, TransactionOpCode,
    TransactionOutpoint, TransactionOutput,
};
use types::HealthCheckResponse;

use rpc::QuibleRpcServer;

pub mod cert;
pub mod db;
pub mod quible_ecdsa_utils;
pub mod quible_transaction_utils;
pub mod rpc;
pub mod tx;
pub mod types;

const SLOT_DURATION: Duration = Duration::from_secs(4);

#[derive(Debug, Deserialize, Serialize)]
struct Record {
    #[allow(dead_code)]
    id: Thing,
}

pub struct QuibleBlockProposerExecutionContextImpl {
    db: Arc<Surreal<AnyDb>>,
    mempool: Vec<([u8; 32], Transaction)>,
    transaction_cache: HashMap<[u8; 32], Transaction>,
    spent_outpoints: Vec<TransactionOutpoint>,
    included_transactions: Vec<[u8; 32]>,
}

#[async_trait]
impl ExecutionContext for QuibleBlockProposerExecutionContextImpl {
    async fn fetch_next_pending_transaction(
        &mut self,
    ) -> anyhow::Result<Option<([u8; 32], Transaction)>> {
        let entry = self.mempool.pop();

        if let Some((transaction_hash, transaction)) = entry.clone() {
            self.transaction_cache.insert(transaction_hash, transaction);
        }

        Ok(entry)
    }

    async fn fetch_unspent_output(
        &mut self,
        outpoint: TransactionOutpoint,
    ) -> anyhow::Result<TransactionOutput> {
        let transaction_hash_hex = hex::encode(outpoint.txid);
        let mut result = self
            .db
            .query("SELECT * FROM transaction_outputs WHERE id = $id")
            .bind((
                "id",
                SurrealID(Thing::from((
                    "transaction_outputs".to_string(),
                    format!(
                        "{}:{}",
                        transaction_hash_hex.clone().to_string(),
                        outpoint.index
                    ),
                ))),
            ))
            .await
            .map_err(|err| anyhow!(err))?;

        let transaction_output_row_maybe: Option<TransactionOutputRow> = result.take(0)?;

        match transaction_output_row_maybe {
            Some(transaction_output_row) => {
                if transaction_output_row.spent {
                    return Err(anyhow!("cannot spend output twice"));
                }

                Ok(transaction_output_row.output)
            }

            None => Err(anyhow!("transaction hash not found!")),
        }
    }

    async fn include_in_next_block(&mut self, transaction_hash: [u8; 32]) -> anyhow::Result<()> {
        let transaction = self
            .transaction_cache
            .get(&transaction_hash)
            .ok_or(anyhow!("transaction hash not found!"))?;
        let Transaction::Version1 { inputs, .. } = transaction;

        for input in inputs {
            self.spent_outpoints.push(input.clone().outpoint);
        }

        self.included_transactions.push(transaction_hash);

        Ok(())
    }

    async fn record_invalid_transaction(
        &mut self,
        transaction_hash: [u8; 32],
        error: anyhow::Error,
    ) -> anyhow::Result<()> {
        dbg!(transaction_hash, error);

        let transaction_hash_hex = hex::encode(transaction_hash);

        self.db
            .query("DELETE FROM pending_transactions WHERE id = $id")
            .bind((
                "id",
                SurrealID(Thing::from((
                    "pending_transactions".to_string(),
                    transaction_hash_hex.clone().to_string(),
                ))),
            ))
            .await?;

        Ok(())
    }
}

async fn digest_object_output(
    db: &Arc<Surreal<AnyDb>>,
    object_id: &ObjectIdentifier,
    data_script: &Vec<TransactionOpCode>,
) -> anyhow::Result<()> {
    let object_id_hex = hex::encode(object_id.raw);
    let surreal_object_id = SurrealID(Thing::from((
        "objects".to_string(),
        object_id_hex.to_string(),
    )));

    if let ObjectMode::Fresh = object_id.mode {
        let _result: Vec<ObjectRow> = db
            .create("objects")
            .content(ObjectRow {
                id: surreal_object_id.clone(),
                object_id: object_id_hex,
                claims: vec![],
                cert_ttl: 86400,
            })
            .await?;
    };

    for opcode in data_script {
        match opcode {
            TransactionOpCode::DeleteAll => {
                db.query("UPDATE objects SET claims = [] WHERE id = $id")
                    .bind(("id", surreal_object_id.clone()))
                    .await?;
            }

            TransactionOpCode::Insert { data } => {
                db.query("UPDATE objects SET claims += $value WHERE id = $id")
                    .bind(("id", surreal_object_id.clone()))
                    .bind(("value", surrealdb::sql::Bytes::from(data.clone())))
                    .await?;
            }

            TransactionOpCode::Delete { data } => {
                db.query("UPDATE objects SET claims -= $value WHERE id = $id")
                    .bind(("id", surreal_object_id.clone()))
                    .bind(("value", surrealdb::sql::Bytes::from(data.clone())))
                    .await?;
            }

            TransactionOpCode::SetCertTTL { data } => {
                db.query("UPDATE objects SET cert_ttl = $value WHERE id = $id")
                    .bind(("id", surreal_object_id.clone()))
                    .bind(("value", data))
                    .await?;
            }

            _ => {}
        }
    }

    Ok(())
}

async fn propose_block(
    block_number: u64,
    db_arc: &Arc<Surreal<AnyDb>>,
) -> anyhow::Result<BlockRow> {
    println!("preparing block {}", block_number);

    let previous_block_header: Option<BlockHeader> = db_arc
        .query("SELECT header FROM blocks WHERE height = $height")
        .bind(("height", block_number - 1))
        .await
        .and_then(|mut response| response.take((0, "header")))?;

    let pending_transaction_rows: Vec<PendingTransactionRow> =
        db_arc.select("pending_transactions").await?;

    let mut execution_context = QuibleBlockProposerExecutionContextImpl {
        transaction_cache: HashMap::new(),
        mempool: pending_transaction_rows
            .iter()
            .map(|tx| {
                Ok((tx.clone().data.hash()?, tx.clone().data))
                    as Result<([u8; 32], Transaction), anyhow::Error>
            })
            .collect::<Result<Vec<([u8; 32], Transaction)>, anyhow::Error>>()?,
        db: db_arc.clone(),
        spent_outpoints: vec![],
        included_transactions: vec![],
    };

    let timestamp: u64 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| ErrorDb::Thrown(format!("Failed to generate timestamp: {}", e).into()))?
        .as_secs();

    collect_valid_block_transactions(&mut execution_context).await?;

    let previous_block_header_hash = previous_block_header.map_or(Ok([0u8; 32]), |h| h.hash())?;

    let block_header = BlockHeader::Version1 {
        previous_block_header_hash,
        merkle_root: [0u8; 32],
        timestamp,
    };

    let block_header_hash = block_header.hash()?;
    let block_header_hash_hex = hex::encode(block_header_hash);

    let mut transactions = execution_context
        .included_transactions
        .iter()
        .map(|transaction_hash| {
            Ok((
                transaction_hash.clone(),
                execution_context
                    .transaction_cache
                    .get(transaction_hash)
                    .ok_or(anyhow!(
                        "cannot find included transaction in transaction cache"
                    ))?
                    .clone(),
            ))
        })
        .collect::<Result<Vec<([u8; 32], Transaction)>, anyhow::Error>>()?;

    let coinbase_transaction = Transaction::Version1 {
        inputs: vec![],
        outputs: vec![TransactionOutput::Value {
            value: 5,

            // TODO: https://linear.app/quible/issue/QUI-103/ensure-coinbase-transactions-can-only-be-spent-by-block-proposer
            pubkey_script: vec![],
        }],
        locktime: 0,
    };

    transactions.insert(0, (coinbase_transaction.hash()?, coinbase_transaction));

    let block_row = BlockRow {
        id: SurrealID(Thing::from((
            "pending_transactions".to_string(),
            block_header_hash_hex.clone().to_string(),
        ))),
        hash: block_header_hash_hex,
        header: block_header,
        height: block_number,
        transactions: transactions.clone(),
    };

    db_arc
        .create::<Vec<BlockRow>>("blocks")
        .content(block_row.clone())
        .await?;

    for (transaction_hash, transaction) in transactions {
        let Transaction::Version1 { outputs, .. } = transaction;

        let transaction_hash_hex = hex::encode(transaction_hash);

        for (index, output) in outputs.iter().enumerate() {
            db_arc
                .create::<Vec<TransactionOutputRow>>("transaction_outputs")
                .content(TransactionOutputRow {
                    id: SurrealID(Thing::from((
                        "transaction_outputs".to_string(),
                        format!("{}:{}", transaction_hash_hex.clone().to_string(), index),
                    ))),
                    transaction_hash: transaction_hash_hex.clone(),
                    output_index: index.try_into()?,
                    output: output.clone(),
                    spent: false,
                })
                .await?;

            match output {
                TransactionOutput::Object {
                    object_id,
                    data_script,
                    ..
                } => {
                    digest_object_output(&db_arc, object_id, data_script).await?;
                }

                _ => {}
            }
        }

        db_arc
            .query("DELETE FROM pending_transactions WHERE id = $id")
            .bind((
                "id",
                SurrealID(Thing::from((
                    "pending_transactions".to_string(),
                    transaction_hash_hex.clone().to_string(),
                ))),
            ))
            .await?;
    }

    Ok(block_row)
}

pub struct QuibleRpcServerImpl {
    db: Arc<Surreal<AnyDb>>,
    node_signer_key: [u8; 32],
}

#[jsonrpsee_async_trait]
impl rpc::QuibleRpcServer for QuibleRpcServerImpl {
    async fn send_transaction(&self, transaction: Transaction) -> Result<(), ErrorObjectOwned> {
        let transaction_hash = transaction.hash().map_err(|err| {
            ErrorObjectOwned::owned::<String>(
                CALL_EXECUTION_FAILED_CODE,
                "call execution failed: failed to compute transaction hash",
                Some(format!("{}", err.root_cause())),
            )
        })?;
        // let transaction_json = serde_json::to_value(&transaction).unwrap();

        let transaction_hash_hex = hex::encode(transaction_hash);
        let result: Result<Vec<PendingTransactionRow>, surrealdb::Error> = self
            .db
            .create("pending_transactions")
            .content(PendingTransactionRow {
                id: SurrealID(Thing::from((
                    "pending_transactions".to_string(),
                    transaction_hash_hex.clone().to_string(),
                ))),

                // TODO: https://linear.app/quible/issue/QUI-99/use-surrealdb-bytes-type-for-storing-hashes
                // hash: surrealdb::sql::Bytes::from(transaction_hash.to_vec()),
                hash: transaction_hash_hex,

                data: transaction.clone(),

                // TODO: https://linear.app/quible/issue/QUI-100/track-transaction-sizes-and-only-pull-small-enough-transactions
                size: 0,
            })
            .await;

        match result {
            Ok(pending_transaction_rows) => {
                if pending_transaction_rows.len() == 0 {
                    Err(ErrorObjectOwned::owned::<String>(
                        CALL_EXECUTION_FAILED_CODE,
                        "call execution failed: transaction already inserted",
                        None,
                    ))
                } else {
                    Ok(())
                }
            }
            Err(error) => Err(ErrorObjectOwned::owned::<String>(
                CALL_EXECUTION_FAILED_CODE,
                "call execution failed: failed to insert",
                Some(error.to_string()),
            )),
        }
    }

    async fn check_health(&self) -> Result<types::HealthCheckResponse, ErrorObjectOwned> {
        Ok(HealthCheckResponse {
            status: "healthy".to_string(),
        })
    }

    async fn request_certificate(
        &self,
        object_id: [u8; 32],
        claim: Vec<u8>,
    ) -> Result<SignedCertificate, ErrorObjectOwned> {
        // let result = self.db.query("SELECT object_id FROM objects WHERE object_id = $id AND claims CONTAINS $value")
        let details = CertificateSigningRequestDetails {
            object_id,
            claim,
            expires_at: std::u64::MAX,
        };

        let hash = details.hash().map_err(|err| {
            ErrorObjectOwned::owned::<String>(
                CALL_EXECUTION_FAILED_CODE,
                "call execution failed: failed to sign",
                Some(err.to_string()),
            )
        })?;

        let signature_raw = sign_message(
            B256::from_slice(&self.node_signer_key),
            FixedBytes::new(hash),
        ).map_err(|err| {
            ErrorObjectOwned::owned::<String>(
                CALL_EXECUTION_FAILED_CODE,
                "call execution failed: failed to sign",
                Some(err.to_string()),
            )
        })?;

        Ok(SignedCertificate {
            details,
            signature: QuibleSignature { raw: signature_raw },
        })
    }
}

async fn run_derive_server(
    node_signer_key: [u8; 32],
    db: &Arc<Surreal<AnyDb>>,
    port: u16,
) -> anyhow::Result<SocketAddr> {
    let cors = CorsLayer::new()
        // Allow `POST` when accessing the resource
        .allow_methods([Method::POST])
        // Allow requests from any origin
        .allow_origin(Any)
        .allow_headers([hyper::header::CONTENT_TYPE]);
    let middleware = tower::ServiceBuilder::new().layer(cors);

    let server = Server::builder()
        .set_http_middleware(middleware)
        .build(format!("0.0.0.0:{}", port).parse::<SocketAddr>()?)
        .await?;

    let addr = server.local_addr()?;
    let handle = server.start(
        QuibleRpcServerImpl {
            db: db.clone(),
            node_signer_key,
        }
        .into_rpc(),
    );

    tokio::spawn(handle.stopped());

    Ok(addr)
}

// TODO: https://linear.app/quible/issue/QUI-49/refactor-entrypoint-for-easier-unit-testing
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let signing_key_hex = match env::var("QUIBLE_SIGNER_KEY").ok() {
        Some(key) => key,
        None => {
            let key_file_path = env::var("QUIBLE_SIGNER_KEY_FILE")
                .expect("no QUIBLE_SIGNER_KEY or QUIBLE_SIGNER_KEY_FILE provided");

            let contents = fs::read(key_file_path.clone())
                .expect(&format!("failed to read file at {key_file_path}"));
            std::str::from_utf8(&contents).unwrap().trim().to_owned()
        }
    };

    assert!(
        signing_key_hex.clone().len() == 64,
        "unexpected length for QUIBLE_SIGNER_KEY"
    );
    let mut signing_key_decoded = [0u8; 32];
    hex::decode_to_slice(signing_key_hex, &mut signing_key_decoded)?;

    let p2p_port: u16 = env::var("QUIBLE_P2P_PORT")
        .unwrap_or_else(|_| "9014".to_owned())
        .parse()?;

    let rpc_port: u16 = env::var("QUIBLE_RPC_PORT")
        .unwrap_or_else(|_| "9013".to_owned())
        .parse()?;

    let endpoint = env::var("QUIBLE_DATABASE_URL").unwrap_or_else(|_| "memory".to_owned());

    let leader_addr = env::var("QUIBLE_LEADER_MULTIADDR").ok();

    // surrealdb init
    let db = any::connect(endpoint).await?;
    db.use_ns("quible").use_db("quible_node").await?;
    db::schema::initialize_db(&db).await?;

    if let None = leader_addr {
        db::schema::initialize_tracker_db(&db).await?;
    }

    let db_arc = Arc::new(db);
    let server_addr = run_derive_server(signing_key_decoded, &db_arc, rpc_port).await?;
    let url = format!("http://{}", server_addr);
    println!("server listening at {}", url);

    let mut block_number = 0u64;
    let mut block_timestamp = Instant::now();

    let keypair: libp2p_identity::ecdsa::Keypair =
        libp2p_identity::ecdsa::SecretKey::try_from_bytes(signing_key_decoded)?.into();

    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(keypair.into())
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_dns()?
        .with_behaviour(|_| ping::Behaviour::default())?
        .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(u64::MAX)))
        .build();

    swarm.listen_on(multiaddr::multiaddr!(Ip4([0, 0, 0, 0]), Tcp(p2p_port)))?;

    let remote_addr = leader_addr
        .clone()
        .map(|url| (url.clone(), url.parse::<multiaddr::Multiaddr>().unwrap()));

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

    let result = propose_block(block_number, &db_arc).await;

    if let Err(e) = result {
        eprintln!("Error in propose_block: {:#?}", e);
    }

    loop {
        select! {
            _ = sleep_until(block_timestamp + SLOT_DURATION) => {
                block_timestamp = block_timestamp + SLOT_DURATION;
                block_number += 1;

                let result = propose_block(block_number, &db_arc).await;

                if let Err(e) = result {
                    eprintln!("Error in propose_block: {:#?}", e);
                }
            }

            event = swarm.select_next_some() => match event {
                SwarmEvent::NewListenAddr { address, .. } => println!("libp2p listening on {address:?}"),
                SwarmEvent::Behaviour(libp2p::ping::Event { peer, result: Ok(_), .. }) => {
                    let timestamp: u64 = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map_err(|e| {
                            ErrorDb::Thrown(format!("Failed to generate timestamp: {}", e).into())
                        })?
                    .as_secs();

                    db_arc.create::<Vec<TrackerPing>>("tracker_pings")
                        .content(TrackerPing {
                            peer_id: peer.to_base58(),
                            timestamp
                        })
                    .await?;
                },

                // TODO(QUI-46): enable debug log level
                SwarmEvent::OutgoingConnectionError { .. } => {
                    panic!("dial failure: {event:?}");
                },

                SwarmEvent::ConnectionClosed { .. } => {
                    if leader_addr.is_some() {
                        panic!("leader connection closed: {event:?}");
                    }
                },

                _ => {
                    // TODO(QUI-46): enable debug log level
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{db, run_derive_server};
    use crate::db::types::{BlockRow, ObjectRow, PendingTransactionRow};
    use crate::propose_block;
    use crate::quible_ecdsa_utils::recover_signer_unchecked;
    use crate::rpc::QuibleRpcClient;
    use crate::tx::engine::compute_object_id;
    use crate::tx::types::{
        Hashable, ObjectIdentifier, ObjectMode, Transaction, TransactionInput, TransactionOpCode,
        TransactionOutpoint, TransactionOutput,
    };
    use alloy_primitives::Address;
    use anyhow::anyhow;
    use jsonrpsee::http_client::HttpClient;
    use std::sync::Arc;
    use surrealdb::engine::any;

    #[tokio::test]
    async fn test_send_transaction() -> anyhow::Result<()> {
        // Initialize SurrealDB
        let db = any::connect("memory").await?;
        db.use_ns("quible").use_db("quible_node").await?;
        db::schema::initialize_db(&db).await?;

        let db_arc = Arc::new(db);

        let server_addr = run_derive_server(
            hex_literal::hex!("ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"),
            &db_arc,
            0,
        )
        .await?;
        let url = format!("http://{}", server_addr);
        println!("server listening at {}", url);
        let client = HttpClient::builder().build(url)?;
        // let signer_secret = k256::ecdsa::SigningKey::random(&mut rand::thread_rng());
        let response = client
            .send_transaction(Transaction::Version1 {
                inputs: vec![],
                outputs: vec![TransactionOutput::Value {
                    value: 0,
                    pubkey_script: vec![],
                }],
                locktime: 0,
            })
            .await
            .unwrap();
        dbg!("response: {:?}", response);

        // Query pending transactions from SurrealDB
        let pending_transaction_rows: Vec<PendingTransactionRow> =
            db_arc.select("pending_transactions").await?;

        assert_eq!(pending_transaction_rows.len(), 1);
        for row in pending_transaction_rows {
            println!("Transaction Hash: {}", row.hash);
            println!(
                "Transaction Data: {}",
                serde_json::to_string_pretty(&row.data)?
            );
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_proposes_block_with_no_transactions() -> anyhow::Result<()> {
        // Initialize SurrealDB
        let db = any::connect("memory").await?;
        db.use_ns("quible").use_db("quible_node").await?;
        db::schema::initialize_db(&db).await?;

        let db_arc = Arc::new(db);

        let server_addr = run_derive_server(
            hex_literal::hex!("ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"),
            &db_arc,
            0,
        )
        .await?;
        let url = format!("http://{}", server_addr);
        println!("server listening at {}", url);

        propose_block(1, &db_arc).await?;

        // Query pending transactions from SurrealDB
        let block_rows: Vec<BlockRow> = db_arc.select("blocks").await?;

        assert_eq!(block_rows.len(), 1);
        for row in block_rows {
            println!("Block Hash: {}", row.hash);
            println!(
                "Block Header: {}",
                serde_json::to_string_pretty(&row.header)?
            );
        }

        Ok(())
    }

    #[tokio::test]
    async fn accepts_valid_transactions_and_excludes_invalid_transactions() -> anyhow::Result<()> {
        // Initialize SurrealDB
        let db = any::connect("memory").await?;
        db.use_ns("quible").use_db("quible_node").await?;
        db::schema::initialize_db(&db).await?;

        let db_arc = Arc::new(db);

        let server_addr = run_derive_server(
            hex_literal::hex!("ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"),
            &db_arc,
            0,
        )
        .await?;
        let url = format!("http://{}", server_addr);
        println!("server listening at {}", url);
        let client = HttpClient::builder().build(url)?;
        // let signer_secret = k256::ecdsa::SigningKey::random(&mut rand::thread_rng());
        let sample_transaction = Transaction::Version1 {
            inputs: vec![],
            outputs: vec![TransactionOutput::Value {
                value: 0,
                pubkey_script: vec![],
            }],
            locktime: 0,
        };

        let sample_invalid_transaction = Transaction::Version1 {
            inputs: vec![TransactionInput {
                outpoint: TransactionOutpoint {
                    txid: [0u8; 32],
                    index: 0,
                },
                signature_script: vec![],
            }],
            outputs: vec![TransactionOutput::Value {
                value: 0,
                pubkey_script: vec![],
            }],
            locktime: 0,
        };

        client
            .send_transaction(sample_transaction.clone())
            .await
            .unwrap();
        client
            .send_transaction(sample_invalid_transaction.clone())
            .await
            .unwrap();

        propose_block(1, &db_arc).await?;

        // Query pending transactions from SurrealDB
        let block_rows: Vec<BlockRow> = db_arc.select("blocks").await?;

        assert_eq!(block_rows.len(), 1);
        for row in block_rows {
            println!("Block Hash: {}", row.hash);
            println!(
                "Block Header: {}",
                serde_json::to_string_pretty(&row.header)?
            );

            match row.transactions[..] {
                [(coinbase_transaction_hash, _), (first_transaction_hash, _)] => {
                    assert_ne!(
                        coinbase_transaction_hash,
                        sample_invalid_transaction.hash()?
                    );
                    assert_eq!(first_transaction_hash, sample_transaction.hash()?);
                    Ok(())
                }

                _ => Err(anyhow!("unexpected number of transactions")),
            }?;
        }

        let pending_transaction_rows: Vec<PendingTransactionRow> =
            db_arc.select("pending_transactions").await?;

        assert_eq!(pending_transaction_rows.len(), 0);

        Ok(())
    }

    #[tokio::test]
    async fn transactions_can_spend_outputs_from_previous_blocks() -> anyhow::Result<()> {
        // Initialize SurrealDB
        let db = any::connect("memory").await?;
        db.use_ns("quible").use_db("quible_node").await?;
        db::schema::initialize_db(&db).await?;

        let db_arc = Arc::new(db);

        let server_addr = run_derive_server(
            hex_literal::hex!("ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"),
            &db_arc,
            0,
        )
        .await?;
        let url = format!("http://{}", server_addr);
        println!("server listening at {}", url);

        let block_row = propose_block(1, &db_arc).await?;

        let coinbase_transaction_hash = match &block_row.transactions[..] {
            [(hash, _)] => Ok(*hash),
            _ => Err(anyhow!("missing coinbase transaction")),
        }?;

        let client = HttpClient::builder().build(url)?;
        // let signer_secret = k256::ecdsa::SigningKey::random(&mut rand::thread_rng());
        let sample_first_transaction = Transaction::Version1 {
            inputs: vec![TransactionInput {
                outpoint: TransactionOutpoint {
                    txid: coinbase_transaction_hash,
                    index: 0,
                },
                signature_script: vec![],
            }],
            outputs: vec![TransactionOutput::Value {
                value: 0,
                pubkey_script: vec![],
            }],
            locktime: 0,
        };

        client
            .send_transaction(sample_first_transaction.clone())
            .await
            .unwrap();

        propose_block(2, &db_arc).await?;

        // Query pending transactions from SurrealDB
        let mut result = db_arc
            .query("SELECT * FROM blocks ORDER BY height ASC")
            .await?;
        let block_rows: Vec<BlockRow> = result.take(0)?;

        match &block_rows[..] {
            [_, second_block_row] => match second_block_row.transactions[..] {
                [_, (transaction_hash, _)] => {
                    assert_eq!(transaction_hash, sample_first_transaction.hash()?);
                    Ok(())
                }

                _ => {
                    dbg!(second_block_row);
                    Err(anyhow!("unexpected number of transactions"))
                }
            },

            _ => Err(anyhow!("unexpected number of block rows")),
        }?;

        let pending_transaction_rows: Vec<PendingTransactionRow> =
            db_arc.select("pending_transactions").await?;

        assert_eq!(pending_transaction_rows.len(), 0);

        Ok(())
    }

    #[tokio::test]
    async fn accepts_transactions_with_object_outputs() -> anyhow::Result<()> {
        // Initialize SurrealDB
        let db = any::connect("memory").await?;
        db.use_ns("quible").use_db("quible_node").await?;
        db::schema::initialize_db(&db).await?;

        let db_arc = Arc::new(db);

        let server_addr = run_derive_server(
            hex_literal::hex!("ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"),
            &db_arc,
            0,
        )
        .await?;
        let url = format!("http://{}", server_addr);
        println!("server listening at {}", url);
        let client = HttpClient::builder().build(url)?;
        // let signer_secret = k256::ecdsa::SigningKey::random(&mut rand::thread_rng());
        let object_id_raw = compute_object_id(vec![], 0)?;
        let sample_transaction = Transaction::Version1 {
            inputs: vec![],
            outputs: vec![TransactionOutput::Object {
                object_id: ObjectIdentifier {
                    raw: object_id_raw,
                    mode: ObjectMode::Fresh,
                },
                data_script: vec![
                    TransactionOpCode::Insert {
                        data: vec![1, 2, 3],
                    },
                    TransactionOpCode::DeleteAll,
                    TransactionOpCode::Insert {
                        data: vec![4, 5, 6],
                    },
                    TransactionOpCode::Delete {
                        data: vec![4, 5, 6],
                    },
                    TransactionOpCode::Insert {
                        data: vec![7, 8, 9],
                    },
                ],
                pubkey_script: vec![],
            }],
            locktime: 0,
        };

        client
            .send_transaction(sample_transaction.clone())
            .await
            .unwrap();

        propose_block(1, &db_arc).await?;

        // Query pending transactions from SurrealDB
        let block_rows: Vec<BlockRow> = db_arc.select("blocks").await?;

        assert_eq!(block_rows.len(), 1);
        for row in block_rows {
            println!("Block Hash: {}", row.hash);
            println!(
                "Block Header: {}",
                serde_json::to_string_pretty(&row.header)?
            );

            match row.transactions[..] {
                [_, (first_transaction_hash, _)] => {
                    assert_eq!(first_transaction_hash, sample_transaction.hash()?);
                    Ok(())
                }

                _ => Err(anyhow!("unexpected number of transactions")),
            }?;
        }

        let pending_transaction_rows: Vec<PendingTransactionRow> =
            db_arc.select("pending_transactions").await?;

        assert_eq!(pending_transaction_rows.len(), 0);

        let object_rows: Vec<ObjectRow> = db_arc.select("objects").await?;

        match &object_rows[..] {
            [object_row] => {
                assert_eq!(object_row.object_id, hex::encode(object_id_raw));
                assert_eq!(object_row.claims, vec![vec![7, 8, 9]]);

                Ok(())
            }

            _ => Err(anyhow!("unexpected number of objects")),
        }?;

        Ok(())
    }

    #[tokio::test]
    async fn issues_valid_certificates_for_valid_requests() -> anyhow::Result<()> {
        // Initialize SurrealDB
        let db = any::connect("memory").await?;
        db.use_ns("quible").use_db("quible_node").await?;
        db::schema::initialize_db(&db).await?;

        let db_arc = Arc::new(db);

        let server_signer_key = k256::ecdsa::SigningKey::from_slice(&hex_literal::hex!(
            "ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
        ))?;

        let server_addr = run_derive_server(
            server_signer_key.to_bytes().as_slice().try_into()?,
            &db_arc,
            0,
        )
        .await?;
        let url = format!("http://{}", server_addr);
        println!("server listening at {}", url);
        let client = HttpClient::builder().build(url)?;
        // let signer_secret = k256::ecdsa::SigningKey::random(&mut rand::thread_rng());
        let object_id_raw = compute_object_id(vec![], 0)?;
        let sample_transaction = Transaction::Version1 {
            inputs: vec![],
            outputs: vec![TransactionOutput::Object {
                object_id: ObjectIdentifier {
                    raw: object_id_raw,
                    mode: ObjectMode::Fresh,
                },
                data_script: vec![TransactionOpCode::Insert {
                    data: vec![1, 2, 3],
                }],
                pubkey_script: vec![],
            }],
            locktime: 0,
        };

        client.send_transaction(sample_transaction.clone()).await?;

        propose_block(1, &db_arc).await?;

        let cert = client
            .request_certificate(object_id_raw, vec![1, 2, 3])
            .await?;

        assert_eq!(cert.details.object_id, object_id_raw);
        assert_eq!(cert.details.claim, vec![1, 2, 3]);
        assert_eq!(
            recover_signer_unchecked(&cert.signature.raw, &cert.details.hash()?)?,
            Address::from_public_key(server_signer_key.verifying_key())
        );

        Ok(())
    }
}
