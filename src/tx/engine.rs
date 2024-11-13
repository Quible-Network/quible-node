use anyhow::anyhow;
use async_trait::async_trait;
use sha3::{Digest, Keccak256};

use crate::{
    quible_ecdsa_utils::recover_signer_unchecked,
    tx::types::{ObjectMode, Transaction, TransactionInput, TransactionOpCode},
};

use super::types::{Hashable, TransactionOutpoint, TransactionOutput};

#[async_trait]
pub trait ExecutionContext {
    // looks for an additional pending transaction from
    // the mempool that is small enough to fit in the
    // remaining space for the block.
    //
    // Remaining space is determined by totaling the size
    // of all included transactions, and subtracting that
    // from that block limit.
    async fn fetch_next_pending_transaction(
        &mut self,
    ) -> anyhow::Result<Option<([u8; 32], Transaction)>>;

    // looks up transaction output by outpoint. If not
    // found, an error is thrown. If found but is spent,
    // an error is thrown.
    async fn fetch_unspent_output(
        &mut self,
        outpoint: TransactionOutpoint,
    ) -> anyhow::Result<TransactionOutput>;

    // marks the transaction as valid in the execution context
    async fn include_in_next_block(&mut self, transaction_hash: [u8; 32]) -> anyhow::Result<()>;

    async fn record_invalid_transaction(
        &mut self,
        transaction_hash: [u8; 32],
        error: anyhow::Error,
    ) -> anyhow::Result<()>;
}

pub fn compute_object_id(
    inputs: Vec<TransactionInput>,
    output_index: u32,
) -> anyhow::Result<[u8; 32]> {
    let mut hasher = Keccak256::new();
    for input in inputs {
        hasher.update(input.outpoint.txid);
        hasher.update(bytemuck::cast::<u32, [u8; 4]>(input.outpoint.index));
    }
    hasher.update(bytemuck::cast::<u32, [u8; 4]>(output_index));

    hasher
        .finalize()
        .as_slice()
        .try_into()
        .map_err(|_| anyhow!("failed to slice Keccak256 hash to 32 bytes"))
}

// pulls pending transactions from context and executes
// them one-by-one until no more pending transactions
// will fit in the block
pub async fn collect_valid_block_transactions<C: ExecutionContext>(
    context: &mut C,
) -> anyhow::Result<()> {
    while let Some((transaction_hash, transaction)) =
        context.fetch_next_pending_transaction().await?
    {
        let Transaction::Version1 {
            inputs, outputs, ..
        } = transaction.clone();

        let execute_transaction = async {
            let mut spent_outpoints = Vec::<TransactionOutpoint>::new();
            let mut input_value = 0u64;
            let mut output_value = 0u64;

            for (
                index,
                TransactionInput {
                    outpoint,
                    signature_script,
                },
            ) in inputs.iter().enumerate()
            {
                dbg!(index, signature_script, spent_outpoints.len());

                if spent_outpoints.contains(&outpoint.clone()) {
                    // TODO: serialize outpoint details for error message
                    return Err(anyhow!("cannot spend output twice"));
                }

                let output_being_spent = context.fetch_unspent_output(outpoint.clone()).await?;
                spent_outpoints.push(outpoint.clone());

                let mut stack: Vec<Vec<u8>> = vec![];

                for opcode in signature_script {
                    match opcode {
                        TransactionOpCode::Push { data } => {
                            stack.push(data.clone());
                        }

                        _ => {
                            return Err(anyhow!("only pushes are allowed in signature scripts"));
                        }
                    }
                }

                let pubkey_script = match output_being_spent.clone() {
                    TransactionOutput::Value { pubkey_script, .. } => pubkey_script,
                    TransactionOutput::Object { pubkey_script, .. } => pubkey_script,
                };

                for opcode in pubkey_script {
                    match opcode {
                        TransactionOpCode::Dup => {
                            let maybe_item = stack.pop();
                            if let Some(item) = maybe_item {
                                stack.push(item.clone());
                                stack.push(item);
                            }
                        }

                        TransactionOpCode::Push { data } => {
                            stack.push(data.clone());
                        }

                        TransactionOpCode::EqualVerify => {
                            let left_maybe = stack.pop();
                            let right_maybe = stack.pop();

                            match (left_maybe, right_maybe) {
                                (Some(left), Some(right)) => {
                                    if left != right {
                                        return Err(anyhow!("pubkey script failed"));
                                    }
                                }

                                _ => {
                                    return Err(anyhow!("pubkey script failed"));
                                }
                            }
                        }

                        TransactionOpCode::CheckSigVerify => {
                            let pubkey_maybe = stack.pop();
                            let sig_maybe = stack.pop();

                            match (pubkey_maybe, sig_maybe) {
                                (Some(pubkey), Some(sig)) => {
                                    let signable_transaction = &mut transaction.to_owned();
                                    match signable_transaction {
                                        Transaction::Version1 { inputs, .. } => {
                                            for input in inputs.iter_mut() {
                                                *input = TransactionInput {
                                                    outpoint: input.clone().outpoint,
                                                    signature_script: vec![],
                                                };
                                            }
                                        }
                                    }

                                    let signable_transaction_hash = signable_transaction.hash()?;
                                    let sig_slice: [u8; 65] = sig.try_into().map_err(|_| {
                                        anyhow!("pubkey script failed (signature is not 65 bytes)")
                                    })?;

                                    let signer = recover_signer_unchecked(
                                        &sig_slice,
                                        &signable_transaction_hash,
                                    )?;
                                    let signer_slice = signer.into_array().to_vec();
                                    if pubkey != signer_slice {
                                        return Err(anyhow!(
                                            "pubkey script failed (signer does not match pubkey)"
                                        ));
                                    }
                                }

                                _ => {
                                    return Err(anyhow!("pubkey script failed"));
                                }
                            }
                        }

                        _ => {}
                    }
                }

                match output_being_spent {
                    TransactionOutput::Value { value, .. } => {
                        input_value += value;
                    }

                    _ => {}
                }
            }

            for (index, output) in outputs.iter().enumerate() {
                dbg!(index, output);

                match output {
                    TransactionOutput::Value { value, .. } => {
                        output_value += value;
                    }

                    TransactionOutput::Object { object_id, .. } => {
                        match object_id.mode {
                            ObjectMode::Fresh => {
                                let expected_object_id =
                                    compute_object_id(inputs.clone(), index.try_into()?)?;

                                if object_id.raw != expected_object_id {
                                    return Err(anyhow!("object id invalid"));
                                }
                            }

                            ObjectMode::Existing { permit_index } => {
                                let permit_index_usize: usize = permit_index.try_into()?;

                                match inputs.clone().get(permit_index_usize) {
                                    Some(input) => {
                                        let output_being_spent = context
                                            .fetch_unspent_output(input.outpoint.clone())
                                            .await?;

                                        match output_being_spent {
                                            TransactionOutput::Value { .. } => {
                                                return Err(anyhow!(
                                                    "non-object output cannot be used as a permit"
                                                ));
                                            }

                                            TransactionOutput::Object {
                                                object_id: permit_object_id,
                                                ..
                                            } => {
                                                if object_id.raw != permit_object_id.raw {
                                                    return Err(anyhow!("object id does not match permitted object id"));
                                                }
                                            }
                                        }
                                    }

                                    None => {
                                        return Err(anyhow!("permit index out of bounds"));
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // TODO: https://linear.app/quible/issue/QUI-93/enforce-transaction-fees
            dbg!(output_value, input_value);
            if output_value > input_value {
                return Err(anyhow!("output value exceeds input value"));
            }

            context.include_in_next_block(transaction_hash).await?;

            Ok(())
        };

        if let Err(error) = execute_transaction.await {
            context
                .record_invalid_transaction(transaction_hash, error)
                .await?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::quible_ecdsa_utils::sign_message;
    use crate::tx::types::{
        Hashable, ObjectIdentifier, ObjectMode, Transaction, TransactionInput, TransactionOpCode,
        TransactionOutpoint, TransactionOutput,
    };
    use alloy_primitives::{Address, B256};
    use anyhow::anyhow;
    use async_trait::async_trait;

    use super::{collect_valid_block_transactions, compute_object_id, ExecutionContext};

    struct TestingExecutionContext {
        pub transaction_map: HashMap<[u8; 32], Transaction>,
        pub mempool: Vec<([u8; 32], Transaction)>,
        pub spent_outpoints: Vec<TransactionOutpoint>,
        pub included_transactions: Vec<[u8; 32]>,
        pub failed_transactions: Vec<([u8; 32], anyhow::Error)>,
    }

    fn create_context(
        history: Vec<Transaction>,
        pending_transactions: Vec<Transaction>,
    ) -> TestingExecutionContext {
        let mut transaction_map: HashMap<[u8; 32], Transaction> = HashMap::new();
        let mempool = pending_transactions
            .iter()
            .map(|tx| (tx.hash().unwrap(), tx.clone()))
            .collect();
        let mut spent_outpoints: Vec<TransactionOutpoint> = Vec::new();

        for transaction in history {
            let transaction_hash = transaction.clone().hash().unwrap();
            transaction_map.insert(transaction_hash, transaction.clone());

            let Transaction::Version1 { inputs, .. } = transaction;
            for input in inputs {
                spent_outpoints.push(input.outpoint);
            }
        }

        TestingExecutionContext {
            transaction_map,
            mempool,
            spent_outpoints,
            included_transactions: vec![],
            failed_transactions: vec![],
        }
    }

    #[async_trait]
    impl ExecutionContext for TestingExecutionContext {
        async fn fetch_next_pending_transaction(
            &mut self,
        ) -> anyhow::Result<Option<([u8; 32], Transaction)>> {
            let entry = self.mempool.pop();

            if let Some((transaction_hash, transaction)) = entry.clone() {
                self.transaction_map.insert(transaction_hash, transaction);
            }

            Ok(entry)
        }

        async fn fetch_unspent_output(
            &mut self,
            outpoint: TransactionOutpoint,
        ) -> anyhow::Result<TransactionOutput> {
            if self.spent_outpoints.contains(&outpoint) {
                return Err(anyhow!("cannot spend output twice"));
            }

            dbg!(&self.transaction_map, &outpoint.txid);
            let transaction = self
                .transaction_map
                .get(&outpoint.txid)
                .ok_or(anyhow!("transaction hash not found!"))?;

            let Transaction::Version1 { outputs, .. } = transaction;
            match outputs.get::<usize>(outpoint.index.try_into().unwrap()) {
                Some(output) => Ok(output.clone()),
                None => Err(anyhow!("outpoint index out of bounds for transaction")),
            }
        }

        async fn include_in_next_block(
            &mut self,
            transaction_hash: [u8; 32],
        ) -> anyhow::Result<()> {
            let transaction = self
                .transaction_map
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
            self.failed_transactions.push((transaction_hash, error));
            Ok(())
        }
    }

    #[tokio::test]
    async fn doublespend_input_by_same_transaction() -> anyhow::Result<()> {
        let coinbase = Transaction::Version1 {
            inputs: vec![],
            outputs: vec![TransactionOutput::Value {
                value: 5,
                pubkey_script: vec![],
            }],
            locktime: 0,
        };

        let coinbase_hash = coinbase.hash()?;

        let mut context = create_context(
            vec![coinbase],
            vec![Transaction::Version1 {
                inputs: vec![
                    TransactionInput {
                        outpoint: TransactionOutpoint {
                            txid: coinbase_hash,
                            index: 0,
                        },
                        signature_script: vec![],
                    },
                    TransactionInput {
                        outpoint: TransactionOutpoint {
                            txid: coinbase_hash,
                            index: 0,
                        },
                        signature_script: vec![],
                    },
                ],
                outputs: vec![],
                locktime: 0,
            }],
        );

        collect_valid_block_transactions(&mut context).await?;

        assert_eq!(context.included_transactions.len(), 0);
        let failure_count = context.failed_transactions.len();
        assert_eq!(failure_count, 1);
        let err = &context.failed_transactions.get(0).unwrap().1;
        assert_eq!(format!("{}", err.root_cause()), "cannot spend output twice");

        Ok(())
    }

    #[tokio::test]
    async fn doublespend_input_by_different_transaction() -> anyhow::Result<()> {
        let coinbase = Transaction::Version1 {
            inputs: vec![],
            outputs: vec![TransactionOutput::Value {
                value: 5,
                pubkey_script: vec![],
            }],
            locktime: 0,
        };

        let coinbase_hash = coinbase.hash()?;

        let mut context = create_context(
            vec![coinbase],
            vec![
                Transaction::Version1 {
                    inputs: vec![TransactionInput {
                        outpoint: TransactionOutpoint {
                            txid: coinbase_hash,
                            index: 0,
                        },
                        signature_script: vec![],
                    }],
                    outputs: vec![],
                    locktime: 0,
                },
                Transaction::Version1 {
                    inputs: vec![TransactionInput {
                        outpoint: TransactionOutpoint {
                            txid: coinbase_hash,
                            index: 0,
                        },
                        signature_script: vec![],
                    }],
                    outputs: vec![],
                    locktime: 0,
                },
            ],
        );

        collect_valid_block_transactions(&mut context).await?;

        assert_eq!(context.included_transactions.len(), 1);
        let failure_count = context.failed_transactions.len();
        assert_eq!(failure_count, 1);
        let err = &context.failed_transactions.get(0).unwrap().1;
        assert_eq!(format!("{}", err.root_cause()), "cannot spend output twice");

        Ok(())
    }

    #[tokio::test]
    async fn output_value_cannot_exceed_input() -> anyhow::Result<()> {
        let coinbase = Transaction::Version1 {
            inputs: vec![],
            outputs: vec![TransactionOutput::Value {
                value: 5,
                pubkey_script: vec![],
            }],
            locktime: 0,
        };

        let coinbase_hash = coinbase.hash()?;

        let mut context = create_context(
            vec![coinbase],
            vec![Transaction::Version1 {
                inputs: vec![TransactionInput {
                    outpoint: TransactionOutpoint {
                        txid: coinbase_hash,
                        index: 0,
                    },
                    signature_script: vec![],
                }],
                outputs: vec![TransactionOutput::Value {
                    value: 6,
                    pubkey_script: vec![],
                }],
                locktime: 0,
            }],
        );

        collect_valid_block_transactions(&mut context).await?;

        assert_eq!(context.included_transactions.len(), 0);
        let failure_count = context.failed_transactions.len();
        assert_eq!(failure_count, 1);
        let err = &context.failed_transactions.get(0).unwrap().1;
        assert_eq!(
            format!("{}", err.root_cause()),
            "output value exceeds input value"
        );

        Ok(())
    }

    #[tokio::test]
    async fn pubkey_script() -> anyhow::Result<()> {
        let signer_secret = k256::ecdsa::SigningKey::random(&mut rand::thread_rng());
        let signer_address = Address::from_private_key(&signer_secret);

        // TODO: https://linear.app/quible/issue/QUI-95/refactor-transaction-signing
        let create_subcontext =
            |include_signature: bool| -> anyhow::Result<TestingExecutionContext> {
                let coinbase = Transaction::Version1 {
                    inputs: vec![],
                    outputs: vec![TransactionOutput::Value {
                        value: 5,
                        pubkey_script: vec![
                            TransactionOpCode::Dup,
                            TransactionOpCode::Push {
                                data: signer_address.into_array().to_vec(),
                            },
                            TransactionOpCode::EqualVerify,
                            TransactionOpCode::CheckSigVerify,
                        ],
                    }],
                    locktime: 0,
                };

                let coinbase_hash = coinbase.hash()?;

                let transaction = &mut Transaction::Version1 {
                    inputs: vec![TransactionInput {
                        outpoint: TransactionOutpoint {
                            txid: coinbase_hash,
                            index: 0,
                        },
                        signature_script: vec![],
                    }],
                    outputs: vec![TransactionOutput::Value {
                        value: 5,
                        pubkey_script: vec![],
                    }],
                    locktime: 0,
                }
                .to_owned();

                let signature = sign_message(
                    B256::from_slice(&signer_secret.to_bytes()[..]),
                    transaction.hash()?.into(),
                )?
                .to_vec();

                if include_signature {
                    match transaction {
                        Transaction::Version1 { inputs, .. } => {
                            for input in inputs.iter_mut() {
                                *input = TransactionInput {
                                    outpoint: input.clone().outpoint,
                                    signature_script: vec![
                                        TransactionOpCode::Push {
                                            data: signature.clone(),
                                        },
                                        TransactionOpCode::Push {
                                            data: signer_address.into_array().to_vec(),
                                        },
                                    ],
                                }
                            }
                        }
                    }
                }

                Ok(create_context(vec![coinbase], vec![transaction.clone()]))
            };

        let mut failure_context = create_subcontext(false)?;

        collect_valid_block_transactions(&mut failure_context).await?;

        assert_eq!(failure_context.included_transactions.len(), 0);
        let failure_count = failure_context.failed_transactions.len();
        assert_eq!(failure_count, 1);
        let err = &failure_context.failed_transactions.get(0).unwrap().1;
        assert_eq!(format!("{}", err.root_cause()), "pubkey script failed");

        let mut context = create_subcontext(true)?;

        collect_valid_block_transactions(&mut context).await?;

        assert_eq!(context.included_transactions.len(), 1);
        let failure_count = context.failed_transactions.len();
        assert_eq!(failure_count, 0);

        Ok(())
    }

    #[tokio::test]
    async fn validates_fresh_object_id() -> anyhow::Result<()> {
        let create_subcontext =
            |include_computed_object_id: bool| -> anyhow::Result<TestingExecutionContext> {
                let coinbase = Transaction::Version1 {
                    inputs: vec![],
                    outputs: vec![TransactionOutput::Value {
                        value: 5,
                        pubkey_script: vec![],
                    }],
                    locktime: 0,
                };

                let coinbase_hash = coinbase.hash()?;

                let inputs = vec![TransactionInput {
                    outpoint: TransactionOutpoint {
                        txid: coinbase_hash,
                        index: 0,
                    },
                    signature_script: vec![],
                }];

                let object_id = ObjectIdentifier {
                    raw: if include_computed_object_id {
                        compute_object_id(inputs.clone(), 0)?
                    } else {
                        [0u8; 32]
                    },
                    mode: ObjectMode::Fresh,
                };

                let transaction = Transaction::Version1 {
                    inputs,
                    outputs: vec![TransactionOutput::Object {
                        object_id,
                        data_script: vec![],
                        pubkey_script: vec![],
                    }],
                    locktime: 0,
                };

                Ok(create_context(vec![coinbase], vec![transaction]))
            };

        let mut failure_context = create_subcontext(false)?;

        collect_valid_block_transactions(&mut failure_context).await?;

        assert_eq!(failure_context.included_transactions.len(), 0);
        let failure_count = failure_context.failed_transactions.len();
        assert_eq!(failure_count, 1);
        let err = &failure_context.failed_transactions.get(0).unwrap().1;
        assert_eq!(format!("{}", err.root_cause()), "object id invalid");

        let mut context = create_subcontext(true)?;

        collect_valid_block_transactions(&mut context).await?;

        assert_eq!(context.included_transactions.len(), 1);
        let failure_count = context.failed_transactions.len();
        assert_eq!(failure_count, 0);

        Ok(())
    }

    #[tokio::test]
    async fn existed_object_id_must_match_permitted_object() -> anyhow::Result<()> {
        let create_subcontext =
            |include_matching_object_id: bool| -> anyhow::Result<TestingExecutionContext> {
                let object_id_raw = compute_object_id(vec![], 0)?;
                let coinbase = Transaction::Version1 {
                    inputs: vec![],
                    outputs: vec![TransactionOutput::Object {
                        object_id: ObjectIdentifier {
                            raw: object_id_raw,
                            mode: ObjectMode::Fresh,
                        },
                        data_script: vec![],
                        pubkey_script: vec![],
                    }],
                    locktime: 0,
                };

                let coinbase_hash = coinbase.hash()?;

                let inputs = vec![TransactionInput {
                    outpoint: TransactionOutpoint {
                        txid: coinbase_hash,
                        index: 0,
                    },
                    signature_script: vec![],
                }];

                let object_id = ObjectIdentifier {
                    raw: if include_matching_object_id {
                        object_id_raw
                    } else {
                        [0u8; 32]
                    },
                    mode: ObjectMode::Existing { permit_index: 0 },
                };

                let transaction = Transaction::Version1 {
                    inputs,
                    outputs: vec![TransactionOutput::Object {
                        object_id,
                        data_script: vec![],
                        pubkey_script: vec![],
                    }],
                    locktime: 0,
                };

                Ok(create_context(vec![coinbase], vec![transaction]))
            };

        let mut failure_context = create_subcontext(false)?;

        collect_valid_block_transactions(&mut failure_context).await?;

        assert_eq!(failure_context.included_transactions.len(), 0);
        let failure_count = failure_context.failed_transactions.len();
        assert_eq!(failure_count, 1);
        let err = &failure_context.failed_transactions.get(0).unwrap().1;
        assert_eq!(
            format!("{}", err.root_cause()),
            "object id does not match permitted object id"
        );

        let mut context = create_subcontext(true)?;

        collect_valid_block_transactions(&mut context).await?;

        assert_eq!(context.included_transactions.len(), 1);
        let failure_count = context.failed_transactions.len();
        assert_eq!(failure_count, 0);

        Ok(())
    }

    #[tokio::test]
    async fn invalidates_out_of_bounds_permit_index() -> anyhow::Result<()> {
        let object_id_raw = compute_object_id(vec![], 0)?;
        let coinbase = Transaction::Version1 {
            inputs: vec![],
            outputs: vec![TransactionOutput::Object {
                object_id: ObjectIdentifier {
                    raw: object_id_raw,
                    mode: ObjectMode::Fresh,
                },
                data_script: vec![],
                pubkey_script: vec![],
            }],
            locktime: 0,
        };

        let coinbase_hash = coinbase.hash()?;

        let inputs = vec![TransactionInput {
            outpoint: TransactionOutpoint {
                txid: coinbase_hash,
                index: 0,
            },
            signature_script: vec![],
        }];

        let object_id = ObjectIdentifier {
            raw: object_id_raw,
            mode: ObjectMode::Existing { permit_index: 1 },
        };

        let transaction = Transaction::Version1 {
            inputs,
            outputs: vec![TransactionOutput::Object {
                object_id,
                data_script: vec![],
                pubkey_script: vec![],
            }],
            locktime: 0,
        };

        let mut context = create_context(vec![coinbase], vec![transaction]);

        collect_valid_block_transactions(&mut context).await?;

        assert_eq!(context.included_transactions.len(), 0);
        let failure_count = context.failed_transactions.len();
        assert_eq!(failure_count, 1);
        let err = &context.failed_transactions.get(0).unwrap().1;
        assert_eq!(
            format!("{}", err.root_cause()),
            "permit index out of bounds"
        );

        Ok(())
    }
}
