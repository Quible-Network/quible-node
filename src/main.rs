use alloy_primitives::{Address, FixedBytes, B256};
use anyhow::anyhow;
use async_trait::async_trait;
use cert::types::{CertificateSigningRequestDetails, QuibleSignature, SignedCertificate};
use db::types::{
    BlockRow, IntermediateFaucetOutputRow, ObjectRow, PendingTransactionRow, SurrealID,
    TrackerPing, TransactionOutputRow,
};
use futures::prelude::stream::StreamExt;
use hex;
use hyper::Method;
use jsonrpsee::core::async_trait as jsonrpsee_async_trait;
use jsonrpsee::types::error::CALL_EXECUTION_FAILED_CODE;
use jsonrpsee::{server::Server, types::ErrorObjectOwned};
use k256::ecdsa::SigningKey;
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
    BlockHeader, Hashable, ObjectIdentifier, ObjectMode, Transaction, TransactionInput,
    TransactionOpCode, TransactionOutpoint, TransactionOutput,
};
use types::{FaucetOutputPayload, HealthCheckResponse, ValueOutputEntry, ValueOutputsPayload};

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

            None => {
                dbg!(outpoint);
                Err(anyhow!("transaction hash not found!"))
            }
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
        let transaction_hash_hex = hex::encode(transaction_hash);

        dbg!(transaction_hash_hex.clone(), error);

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
    node_signing_key: &SigningKey,
) -> anyhow::Result<BlockRow> {
    println!("preparing block {}", block_number);

    let previous_block_header: Option<BlockHeader> = if block_number > 0 {
        db_arc
            .query("SELECT header FROM blocks WHERE height = $height")
            .bind(("height", block_number - 1))
            .await
            .and_then(|mut response| response.take((0, "header")))?
    } else {
        None
    };

    let pending_transaction_rows: Vec<PendingTransactionRow> =
        db_arc.select("pending_transactions").await?;

    let mut execution_context = QuibleBlockProposerExecutionContextImpl {
        transaction_cache: HashMap::new(),
        mempool: pending_transaction_rows
            .iter()
            .map(|tx| {
                Ok((tx.clone().data.hash_eip191()?, tx.clone().data))
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
        inputs: vec![TransactionInput {
            outpoint: TransactionOutpoint {
                txid: [0u8; 32],
                index: 0,
            },
            signature_script: vec![
                // we use this so that the transaction hash is unique for each block
                TransactionOpCode::Push {
                    data: previous_block_header_hash.to_vec(),
                },
            ],
        }],

        outputs: vec![TransactionOutput::Value {
            value: 5,

            pubkey_script: vec![
                TransactionOpCode::Dup,
                TransactionOpCode::Push {
                    data: Address::from_private_key(&node_signing_key)
                        .into_array()
                        .to_vec(),
                },
                TransactionOpCode::EqualVerify,
                TransactionOpCode::CheckEip191SigVerify,
            ],
        }],
        locktime: 0,
    };

    transactions.insert(
        0,
        (coinbase_transaction.hash_eip191()?, coinbase_transaction),
    );

    let block_row = BlockRow {
        id: SurrealID(Thing::from((
            "blocks".to_string(),
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
            let (output_type, pubkey_script) = match output {
                TransactionOutput::Object { pubkey_script, .. } => ("Object", pubkey_script),
                TransactionOutput::Value { pubkey_script, .. } => ("Value", pubkey_script),
            };

            let owner = match &pubkey_script[..] {
                [TransactionOpCode::Dup, TransactionOpCode::Push { data: address_vec }, TransactionOpCode::EqualVerify, TransactionOpCode::CheckEip191SigVerify] => {
                    hex::encode(address_vec.as_slice())
                }
                _ => "".to_string(),
            };

            db_arc
                .create::<Vec<TransactionOutputRow>>("transaction_outputs")
                .content(TransactionOutputRow {
                    id: SurrealID(Thing::from((
                        "transaction_outputs".to_string(),
                        format!("{}:{}", transaction_hash_hex.clone().to_string(), index),
                    ))),
                    transaction_hash: transaction_hash_hex.clone(),
                    output_index: index.try_into()?,
                    output_type: output_type.to_string(),
                    output: output.clone(),
                    owner,
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

fn format_pending_transaction_row(
    transaction: Transaction,
) -> Result<([u8; 32], PendingTransactionRow), ErrorObjectOwned> {
    let transaction_hash = transaction.hash_eip191().map_err(|err| {
        ErrorObjectOwned::owned::<String>(
            CALL_EXECUTION_FAILED_CODE,
            "call execution failed: failed to compute transaction hash",
            Some(format!("{}", err.root_cause())),
        )
    })?;
    // let transaction_json = serde_json::to_value(&transaction).unwrap();

    let transaction_hash_hex = hex::encode(transaction_hash);
    Ok((
        transaction_hash,
        PendingTransactionRow {
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
        },
    ))
}

#[jsonrpsee_async_trait]
impl rpc::QuibleRpcServer for QuibleRpcServerImpl {
    async fn send_transaction(&self, transaction: Transaction) -> Result<(), ErrorObjectOwned> {
        let (_, pending_transaction_row) = format_pending_transaction_row(transaction)?;

        let result: Result<Vec<PendingTransactionRow>, surrealdb::Error> = self
            .db
            .create("pending_transactions")
            .content(pending_transaction_row)
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

    async fn send_raw_transaction(&self, raw_transaction: String) -> Result<(), ErrorObjectOwned> {
        let raw_transaction_vec = hex::decode(raw_transaction).map_err(|err| {
            ErrorObjectOwned::owned::<String>(
                CALL_EXECUTION_FAILED_CODE,
                "call execution failed: failed to decode hexadecimal for transaction",
                Some(err.to_string()),
            )
        })?;

        let transaction_result = postcard::from_bytes(&raw_transaction_vec.as_slice());

        let transaction = transaction_result.map_err(|err| {
            ErrorObjectOwned::owned::<String>(
                CALL_EXECUTION_FAILED_CODE,
                "call execution failed: failed to decode transaction bytes",
                Some(err.to_string()),
            )
        })?;

        self.send_transaction(transaction).await
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
        let object_id_hex = hex::encode(object_id);
        let surreal_object_id = SurrealID(Thing::from((
            "objects".to_string(),
            object_id_hex.to_string(),
        )));

        let result = self
            .db
            .query("SELECT object_id FROM objects WHERE id = $id AND claims CONTAINS $value")
            .bind(("id", surreal_object_id))
            .bind(("value", surrealdb::sql::Bytes::from(claim.clone())))
            .await;

        let validity: Option<String> = result
            .and_then(|mut response| response.take((0, "object_id")))
            .map_err(|err| {
                ErrorObjectOwned::owned(
                    CALL_EXECUTION_FAILED_CODE,
                    "call execution failed: database query error",
                    Some(err.to_string()),
                )
            })?;

        validity.ok_or(ErrorObjectOwned::owned(
            CALL_EXECUTION_FAILED_CODE,
            "call execution failed: could not find identity or claim",
            None as Option<String>,
        ))?;

        let details = CertificateSigningRequestDetails {
            object_id,
            claim,

            // TODO: https://linear.app/quible/issue/QUI-107/generate-expiration-dates
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
        )
        .map_err(|err| {
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

    async fn fetch_unspent_value_outputs_by_owner(
        &self,
        owner_address: [u8; 20],
    ) -> Result<ValueOutputsPayload, ErrorObjectOwned> {
        let owner_address_hex = hex::encode(owner_address);
        let result = self.db
            .query("SELECT * FROM transaction_outputs WHERE owner = $owner AND spent = false AND output_type = \"Value\"")
            .bind(("owner", owner_address_hex))
            .await;

        let output_rows: Vec<TransactionOutputRow> = result
            .and_then(|mut response| response.take(0))
            .map_err(|err| {
                ErrorObjectOwned::owned(
                    CALL_EXECUTION_FAILED_CODE,
                    "call execution failed: failed to fetch unspent value outputs",
                    Some(err.to_string()),
                )
            })?;

        let mut total_value = 0u64;

        let mut output_entries: Vec<ValueOutputEntry> = vec![];

        for output_row in output_rows {
            let transaction_hash_vec: Vec<u8> =
                hex::decode(output_row.transaction_hash).map_err(|err| {
                    ErrorObjectOwned::owned(
                        CALL_EXECUTION_FAILED_CODE,
                        "call execution failed: failed to decode transaction hash",
                        Some(err.to_string()),
                    )
                })?;

            let transaction_hash: [u8; 32] = transaction_hash_vec.try_into().map_err(|_| {
                ErrorObjectOwned::owned(
                    CALL_EXECUTION_FAILED_CODE,
                    "call execution failed: transaction hash is not 32 bytes",
                    None as Option<String>,
                )
            })?;

            match output_row.output {
                TransactionOutput::Value { value, .. } => {
                    total_value += value;

                    output_entries.push(ValueOutputEntry {
                        outpoint: TransactionOutpoint {
                            txid: transaction_hash,
                            index: output_row.output_index,
                        },
                        value,
                    })
                }

                _ => {}
            }
        }

        Ok(ValueOutputsPayload {
            total_value,
            outputs: output_entries,
        })
    }

    async fn request_faucet_output(&self) -> Result<FaucetOutputPayload, ErrorObjectOwned> {
        let result = self
            .db
            .query("SELECT * FROM intermediate_faucet_outputs ORDER BY timestamp DESC LIMIT 1")
            .await;

        let output_rows: Vec<IntermediateFaucetOutputRow> = result
            .and_then(|mut response| response.take(0))
            .map_err(|err| {
                ErrorObjectOwned::owned(
                    CALL_EXECUTION_FAILED_CODE,
                    "call execution failed: database query error",
                    Some(err.to_string()),
                )
            })?;

        match &output_rows[..] {
            [IntermediateFaucetOutputRow {
                transaction_hash_hex,
                output_index,
                owner_signing_key_hex,
                ..
            }, ..] => {
                let mut transaction_hash = [0u8; 32];
                hex::decode_to_slice(transaction_hash_hex, &mut transaction_hash).map_err(
                    |err| {
                        ErrorObjectOwned::owned(
                            CALL_EXECUTION_FAILED_CODE,
                            "call execution failed: failed to decode transaction hash hex",
                            Some(err.to_string()),
                        )
                    },
                )?;

                let mut owner_signing_key_array = [0u8; 32];
                hex::decode_to_slice(owner_signing_key_hex, &mut owner_signing_key_array).map_err(
                    |err| {
                        ErrorObjectOwned::owned(
                            CALL_EXECUTION_FAILED_CODE,
                            "call execution failed: failed to decode owner signing key hex",
                            Some(err.to_string()),
                        )
                    },
                )?;

                generate_intermediate_faucet_output(&self).await?;

                Ok(FaucetOutputPayload {
                    outpoint: TransactionOutpoint {
                        txid: transaction_hash,
                        index: *output_index,
                    },
                    value: 0, // TODO
                    owner_signing_key: owner_signing_key_array,
                })
            }

            _ => Err(ErrorObjectOwned::owned(
                CALL_EXECUTION_FAILED_CODE,
                "call execution failed: failed to find existing intermediate faucet output",
                None as Option<String>,
            )),
        }
    }
}

async fn generate_intermediate_faucet_output(
    server: &QuibleRpcServerImpl,
) -> Result<(), ErrorObjectOwned> {
    let temporary_owner_signing_key = k256::ecdsa::SigningKey::random(&mut rand::thread_rng());
    let temporary_owner_signing_key_hex = hex::encode(temporary_owner_signing_key.to_bytes());
    let temporary_owner_address = Address::from_private_key(&temporary_owner_signing_key);

    // TODO: https://linear.app/quible/issue/QUI-111/refactor-db-schema-and-signing-key-usage
    //
    // parsing the signing key like this should happen
    // before the QuibleRpcServerImpl is instantiated
    let node_signing_key = SigningKey::from_slice(&server.node_signer_key).map_err(|err| {
        ErrorObjectOwned::owned(
            CALL_EXECUTION_FAILED_CODE,
            "call execution failed: failed to parse node signer key",
            Some(err.to_string()),
        )
    })?;

    let owner_address = Address::from_private_key(&node_signing_key);
    let owner_address_hex = hex::encode(owner_address);
    let result = server
        .db
        .query(
            "\n\
                SELECT * FROM transaction_outputs\n\
                WHERE owner = $owner\n\
                AND spent = false\n\
                AND output_type = \"Value\"\n\
                AND count(<-spending<-intermediate_faucet_outputs) = 0
                LIMIT 1",
        )
        .bind(("owner", owner_address_hex))
        .await;

    let output_rows: Vec<TransactionOutputRow> = result
        .and_then(|mut response| response.take(0))
        .map_err(|err| {
        ErrorObjectOwned::owned(
            CALL_EXECUTION_FAILED_CODE,
            "call execution failed: failed to fetch unspent value outputs",
            Some(err.to_string()),
        )
    })?;

    let missing_output_err = ErrorObjectOwned::owned(
        CALL_EXECUTION_FAILED_CODE,
        "call execution failed: no value outputs available",
        None as Option<String>,
    );

    let (origin_outpoint, origin_output) = match &output_rows[..] {
        [row, ..] => {
            let mut transaction_hash = [0u8; 32];
            hex::decode_to_slice(row.transaction_hash.clone(), &mut transaction_hash).map_err(
                |err| {
                    ErrorObjectOwned::owned(
                        CALL_EXECUTION_FAILED_CODE,
                        "call execution failed: failed to decode transaction hash hex",
                        Some(err.to_string()),
                    )
                },
            )?;

            let outpoint = TransactionOutpoint {
                txid: transaction_hash,
                index: row.output_index,
            };

            Ok((outpoint, row.output.clone()))
        }

        _ => Err(missing_output_err.clone()),
    }?;

    let TransactionOutput::Value { value, .. } = origin_output else {
        return Err(missing_output_err);
    };

    let unsigned_intermediate_faucet_transaction_inputs = vec![TransactionInput {
        outpoint: origin_outpoint.clone(),
        signature_script: vec![],
    }];

    let unsigned_intermediate_faucet_transaction_outputs = vec![TransactionOutput::Value {
        value,
        pubkey_script: vec![
            TransactionOpCode::Dup,
            TransactionOpCode::Push {
                data: temporary_owner_address.into_array().to_vec(),
            },
            TransactionOpCode::EqualVerify,
            TransactionOpCode::CheckEip191SigVerify,
        ],
    }];

    let unsigned_intermediate_faucet_transaction = Transaction::Version1 {
        inputs: unsigned_intermediate_faucet_transaction_inputs.clone(),
        outputs: unsigned_intermediate_faucet_transaction_outputs.clone(),
        locktime: 0,
    };

    let unsigned_intermediate_faucet_transaction_hash = unsigned_intermediate_faucet_transaction
        .hash_eip191()
        .map_err(|err| {
            ErrorObjectOwned::owned(
                CALL_EXECUTION_FAILED_CODE,
                "call execution failed: failed to compute transaction hash",
                Some(err.to_string()),
            )
        })?;

    let signature = sign_message(
        B256::from_slice(&node_signing_key.to_bytes()[..]),
        unsigned_intermediate_faucet_transaction_hash.into(),
    )
    .map_err(|err| {
        ErrorObjectOwned::owned(
            CALL_EXECUTION_FAILED_CODE,
            "call execution failed: failed to sign transaction",
            Some(err.to_string()),
        )
    })?
    .to_vec();

    let signed_intermediate_faucet_transaction = Transaction::Version1 {
        inputs: unsigned_intermediate_faucet_transaction_inputs
            .iter()
            .map(|input| TransactionInput {
                outpoint: input.outpoint.clone(),
                signature_script: vec![
                    TransactionOpCode::Push {
                        data: signature.clone(),
                    },
                    TransactionOpCode::Push {
                        data: owner_address.into_array().to_vec(),
                    },
                ],
            })
            .collect(),
        outputs: unsigned_intermediate_faucet_transaction_outputs,
        locktime: 0,
    };

    let (_, signed_intermediate_faucet_transaction_row) =
        format_pending_transaction_row(signed_intermediate_faucet_transaction)?;

    let signed_intermediate_faucet_transaction_row_hash_hex =
        signed_intermediate_faucet_transaction_row.clone().hash;

    let _ = server
        .db
        .query(
            "
                BEGIN;
                INSERT INTO pending_transactions $transaction;
                CREATE $faucet_transaction_id SET
                  transaction_hash_hex = $transaction_hash_hex,
                  output_index = 0,
                  owner_signing_key_hex = $owner_signing_key_hex,
                  timestamp = time::now();
                RELATE $faucet_transaction_id->spending->$origin_transaction_output_id;
                COMMIT;
            ",
        )
        .bind((
            "transaction",
            signed_intermediate_faucet_transaction_row.clone(),
        ))
        .bind((
            "transaction_hash_hex",
            signed_intermediate_faucet_transaction_row_hash_hex.clone(),
        ))
        .bind((
            "faucet_transaction_id",
            SurrealID(Thing::from((
                "intermediate_faucet_outputs".to_string(),
                format!(
                    "{}:{}",
                    signed_intermediate_faucet_transaction_row_hash_hex, 0
                ),
            ))),
        ))
        .bind(("owner_signing_key_hex", temporary_owner_signing_key_hex))
        .bind((
            "origin_transaction_output_id",
            SurrealID(Thing::from((
                "transaction_outputs".to_string(),
                format!(
                    "{}:{}",
                    hex::encode(origin_outpoint.clone().txid),
                    origin_outpoint.index
                ),
            ))),
        ))
        .await
        .map_err(|err| {
            ErrorObjectOwned::owned(
                CALL_EXECUTION_FAILED_CODE,
                "call execution failed: failed to insert intermediate faucet transaction data",
                Some(err.to_string()),
            )
        })?;

    Ok(())
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
    let signing_key = SigningKey::from_slice(&signing_key_decoded)?;

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

    let result = propose_block(block_number, &db_arc, &signing_key).await;

    if let Err(e) = result {
        eprintln!("Error in propose_block: {:#?}", e);
    }

    generate_intermediate_faucet_output(&QuibleRpcServerImpl {
        db: db_arc.clone(),
        node_signer_key: signing_key_decoded,
    })
    .await?;

    loop {
        select! {
            _ = sleep_until(block_timestamp + SLOT_DURATION) => {
                block_timestamp = block_timestamp + SLOT_DURATION;
                block_number += 1;

                let result = propose_block(block_number, &db_arc, &signing_key).await;

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
    use crate::quible_ecdsa_utils::{recover_signer_unchecked, sign_message};
    use crate::rpc::QuibleRpcClient;
    use crate::tx::engine::compute_object_id;
    use crate::tx::types::{
        Hashable, ObjectIdentifier, ObjectMode, Transaction, TransactionInput, TransactionOpCode,
        TransactionOutpoint, TransactionOutput,
    };
    use crate::{generate_intermediate_faucet_output, propose_block, QuibleRpcServerImpl};
    use alloy_primitives::{Address, B256};
    use anyhow::anyhow;
    use jsonrpsee::http_client::HttpClient;
    use k256::ecdsa::SigningKey;
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

        let node_signing_key_bytes =
            hex_literal::hex!("ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80");
        let node_signing_key = SigningKey::from_slice(&node_signing_key_bytes)?;

        let server_addr = run_derive_server(node_signing_key_bytes, &db_arc, 0).await?;
        let url = format!("http://{}", server_addr);
        println!("server listening at {}", url);

        propose_block(1, &db_arc, &node_signing_key).await?;

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

        let node_signing_key_bytes =
            hex_literal::hex!("ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80");
        let node_signing_key = SigningKey::from_slice(&node_signing_key_bytes)?;

        let server_addr = run_derive_server(node_signing_key_bytes, &db_arc, 0).await?;
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

        propose_block(1, &db_arc, &node_signing_key).await?;

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
                        sample_invalid_transaction.hash_eip191()?
                    );
                    assert_eq!(first_transaction_hash, sample_transaction.hash_eip191()?);
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

        let node_signing_key_bytes =
            hex_literal::hex!("ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80");
        let node_signing_key = SigningKey::from_slice(&node_signing_key_bytes)?;

        let server_addr = run_derive_server(node_signing_key_bytes, &db_arc, 0).await?;
        let url = format!("http://{}", server_addr);
        println!("server listening at {}", url);

        let block_row = propose_block(1, &db_arc, &node_signing_key).await?;

        let coinbase_transaction_hash = match &block_row.transactions[..] {
            [(hash, _)] => Ok(*hash),
            _ => Err(anyhow!("missing coinbase transaction")),
        }?;

        let client = HttpClient::builder().build(url)?;

        let sample_first_transaction = &mut Transaction::Version1 {
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

        let signature = sign_message(
            B256::from_slice(&node_signing_key_bytes),
            sample_first_transaction.hash_eip191()?.into(),
        )?
        .to_vec();

        match sample_first_transaction {
            Transaction::Version1 { inputs, .. } => {
                for input in inputs.iter_mut() {
                    *input = TransactionInput {
                        outpoint: input.clone().outpoint,
                        signature_script: vec![
                            TransactionOpCode::Push {
                                data: signature.clone(),
                            },
                            TransactionOpCode::Push {
                                data: Address::from_private_key(&node_signing_key)
                                    .into_array()
                                    .to_vec(),
                            },
                        ],
                    }
                }
            }
        }

        client
            .send_transaction(sample_first_transaction.clone())
            .await
            .unwrap();

        propose_block(2, &db_arc, &node_signing_key).await?;

        // Query pending transactions from SurrealDB
        let mut result = db_arc
            .query("SELECT * FROM blocks ORDER BY height ASC")
            .await?;
        let block_rows: Vec<BlockRow> = result.take(0)?;

        match &block_rows[..] {
            [_, second_block_row] => match second_block_row.transactions[..] {
                [_, (transaction_hash, _)] => {
                    assert_eq!(transaction_hash, sample_first_transaction.hash_eip191()?);
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

        let node_signing_key_bytes =
            hex_literal::hex!("ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80");
        let node_signing_key = SigningKey::from_slice(&node_signing_key_bytes)?;

        let server_addr = run_derive_server(node_signing_key_bytes, &db_arc, 0).await?;
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

        propose_block(1, &db_arc, &node_signing_key).await?;

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
                    assert_eq!(first_transaction_hash, sample_transaction.hash_eip191()?);
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

        let server_signing_key_bytes =
            hex_literal::hex!("ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80");

        let server_signing_key = k256::ecdsa::SigningKey::from_slice(&server_signing_key_bytes)?;

        let server_addr = run_derive_server(server_signing_key_bytes, &db_arc, 0).await?;

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

        propose_block(1, &db_arc, &server_signing_key).await?;

        let cert = client
            .request_certificate(object_id_raw, vec![1, 2, 3])
            .await?;

        assert_eq!(cert.details.object_id, object_id_raw);
        assert_eq!(cert.details.claim, vec![1, 2, 3]);
        assert_eq!(
            recover_signer_unchecked(&cert.signature.raw, &cert.details.hash()?)?,
            Address::from_private_key(&server_signing_key)
        );

        Ok(())
    }

    #[tokio::test]
    async fn refuses_issuance_when_claim_is_missing() -> anyhow::Result<()> {
        // Initialize SurrealDB
        let db = any::connect("memory").await?;
        db.use_ns("quible").use_db("quible_node").await?;
        db::schema::initialize_db(&db).await?;

        let db_arc = Arc::new(db);

        let server_signing_key_bytes =
            hex_literal::hex!("ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80");
        let server_signing_key = k256::ecdsa::SigningKey::from_slice(&server_signing_key_bytes)?;

        let server_addr = run_derive_server(
            server_signing_key.to_bytes().as_slice().try_into()?,
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

        propose_block(1, &db_arc, &server_signing_key).await?;

        let failure_response = client
            .request_certificate(object_id_raw, vec![4, 5, 6])
            .await;

        match failure_response {
            Err(jsonrpsee::core::client::error::Error::Call(err)) => {
                assert_eq!(
                    err.message(),
                    "call execution failed: could not find identity or claim"
                );
                Ok(())
            }

            _ => Err(anyhow!("expected response to be Err(Call(_))")),
        }
    }

    #[tokio::test]
    async fn fetches_unspent_value_outputs() -> anyhow::Result<()> {
        // Initialize SurrealDB
        let db = any::connect("memory").await?;
        db.use_ns("quible").use_db("quible_node").await?;
        db::schema::initialize_db(&db).await?;

        let db_arc = Arc::new(db);

        let server_signing_key_bytes =
            hex_literal::hex!("ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80");

        let server_signing_key = k256::ecdsa::SigningKey::from_slice(&server_signing_key_bytes)?;
        let server_signing_key_bytes: [u8; 32] =
            server_signing_key.to_bytes().as_slice().try_into()?;

        let user_signing_key = k256::ecdsa::SigningKey::random(&mut rand::thread_rng());

        let server_addr = run_derive_server(server_signing_key_bytes, &db_arc, 0).await?;
        let url = format!("http://{}", server_addr);
        println!("server listening at {}", url);
        let client = HttpClient::builder().build(url)?;

        let block_row = propose_block(1, &db_arc, &server_signing_key).await?;

        let coinbase_transaction_hash = match &block_row.transactions[..] {
            [(hash, _)] => Ok(*hash),
            _ => Err(anyhow!("missing coinbase transaction")),
        }?;

        let owner_address = Address::from_private_key(&server_signing_key.clone());
        let user_address = Address::from_private_key(&user_signing_key.clone());

        let sample_transaction = &mut Transaction::Version1 {
            inputs: vec![TransactionInput {
                outpoint: TransactionOutpoint {
                    txid: coinbase_transaction_hash,
                    index: 0,
                },
                signature_script: vec![],
            }],
            outputs: vec![TransactionOutput::Value {
                value: 5,
                pubkey_script: vec![
                    TransactionOpCode::Dup,
                    TransactionOpCode::Push {
                        data: user_address.to_vec(),
                    },
                    TransactionOpCode::EqualVerify,
                    TransactionOpCode::CheckEip191SigVerify,
                ],
            }],
            locktime: 0,
        };

        let signature = sign_message(
            B256::from_slice(&server_signing_key_bytes),
            sample_transaction.hash_eip191()?.into(),
        )?
        .to_vec();

        match sample_transaction {
            Transaction::Version1 { inputs, .. } => {
                for input in inputs.iter_mut() {
                    *input = TransactionInput {
                        outpoint: input.clone().outpoint,
                        signature_script: vec![
                            TransactionOpCode::Push {
                                data: signature.clone(),
                            },
                            TransactionOpCode::Push {
                                data: owner_address.into_array().to_vec(),
                            },
                        ],
                    }
                }
            }
        }

        client.send_transaction(sample_transaction.clone()).await?;

        propose_block(2, &db_arc, &server_signing_key).await?;

        let payload = client
            .fetch_unspent_value_outputs_by_owner(user_address.into_array())
            .await?;

        assert_eq!(payload.total_value, 5);

        match &payload.outputs[..] {
            [output_row] => {
                assert_eq!(
                    output_row.outpoint.txid,
                    sample_transaction.clone().hash_eip191()?
                );

                Ok(())
            }

            _ => {
                dbg!(payload.clone().outputs);
                Err(anyhow!("unexpected number of outputs"))
            }
        }?;

        Ok(())
    }

    #[tokio::test]
    async fn fails_to_find_faucet_outputs_when_none_are_generated() -> anyhow::Result<()> {
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

        let _ = propose_block(1, &db_arc, &server_signer_key).await?;

        let result = client.request_faucet_output().await;

        match result {
            Err(jsonrpsee::core::client::error::Error::Call(err)) => {
                assert_eq!(
                    err.message(),
                    "call execution failed: failed to find existing intermediate faucet output",
                );
                Ok(())
            }

            _ => Err(anyhow!("expected response to be Err(Call(_))")),
        }
    }

    #[tokio::test]
    async fn generates_spendable_faucet_outputs() -> anyhow::Result<()> {
        // Initialize SurrealDB
        let db = any::connect("memory").await?;
        db.use_ns("quible").use_db("quible_node").await?;
        db::schema::initialize_db(&db).await?;

        let db_arc = Arc::new(db);

        let server_signer_key_bytes =
            hex_literal::hex!("ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80");

        let server_signer_key = k256::ecdsa::SigningKey::from_slice(&server_signer_key_bytes)?;

        let faucet_user_signing_key = k256::ecdsa::SigningKey::random(&mut rand::thread_rng());
        let faucet_user_address = Address::from_private_key(&faucet_user_signing_key);

        let server_addr = run_derive_server(server_signer_key_bytes, &db_arc, 0).await?;
        let url = format!("http://{}", server_addr);
        println!("server listening at {}", url);
        let client = HttpClient::builder().build(url)?;

        let _ = propose_block(1, &db_arc, &server_signer_key).await?;

        generate_intermediate_faucet_output(&QuibleRpcServerImpl {
            db: db_arc.clone(),
            node_signer_key: server_signer_key_bytes,
        })
        .await?;

        // extra block proposal ensures that the
        // intermediate transaction is executed
        let _ = propose_block(2, &db_arc, &server_signer_key).await?;

        let faucet_payload = client.request_faucet_output().await?;

        let faucet_owner_signing_key_bytes = faucet_payload.owner_signing_key;
        let faucet_owner_address =
            Address::from_private_key(&SigningKey::from_slice(&faucet_owner_signing_key_bytes)?);

        let sample_transaction = &mut Transaction::Version1 {
            inputs: vec![TransactionInput {
                outpoint: faucet_payload.outpoint,
                signature_script: vec![],
            }],
            outputs: vec![TransactionOutput::Value {
                value: 5,
                pubkey_script: vec![
                    TransactionOpCode::Dup,
                    TransactionOpCode::Push {
                        data: faucet_user_address.into_array().to_vec(),
                    },
                    TransactionOpCode::EqualVerify,
                    TransactionOpCode::CheckEip191SigVerify,
                ],
            }],
            locktime: 0,
        };

        let signature = sign_message(
            B256::from_slice(&faucet_owner_signing_key_bytes),
            sample_transaction.hash_eip191()?.into(),
        )?
        .to_vec();

        match sample_transaction {
            Transaction::Version1 { inputs, .. } => {
                for input in inputs.iter_mut() {
                    *input = TransactionInput {
                        outpoint: input.clone().outpoint,
                        signature_script: vec![
                            TransactionOpCode::Push {
                                data: signature.clone(),
                            },
                            TransactionOpCode::Push {
                                data: faucet_owner_address.into_array().to_vec(),
                            },
                        ],
                    }
                }
            }
        }

        client.send_transaction(sample_transaction.clone()).await?;

        propose_block(3, &db_arc, &server_signer_key).await?;

        let payload = client
            .fetch_unspent_value_outputs_by_owner(faucet_user_address.into_array())
            .await?;

        assert_eq!(payload.total_value, 5);

        match &payload.outputs[..] {
            [output_row] => {
                assert_eq!(
                    output_row.outpoint.txid,
                    sample_transaction.clone().hash_eip191()?
                );

                Ok(())
            }

            _ => {
                dbg!(payload.clone().outputs);
                Err(anyhow!("unexpected number of outputs"))
            }
        }?;

        Ok(())
    }
}
