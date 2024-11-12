use anyhow::anyhow;
use async_trait::async_trait;
use sha3::{Digest, Keccak256};

use crate::tx::types::{ObjectMode, Transaction, TransactionInput};

use super::types::{ObjectIdentifier, TransactionOutpoint, TransactionOutput};

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

// pulls pending transactions from context and executes
// them one-by-one until no more pending transactions
// will fit in the block
pub async fn collect_valid_block_transactions<C: ExecutionContext>(
    context: &mut C,
) -> anyhow::Result<()> {
    while let Some((transaction_hash, transaction)) =
        context.fetch_next_pending_transaction().await?
    {
        let Transaction::Version1 { inputs, .. } = transaction;

        let execute_transaction = async {
            let mut spent_outpoints = Vec::<TransactionOutpoint>::new();

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

                let _output_being_spent = context.fetch_unspent_output(outpoint.clone()).await?;
                spent_outpoints.push(outpoint.clone());

                // TODO: verify signature script has only data pushes
                // TODO: execute signature script followed by input's pubkey script
                // TODO: accumulate input's value
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

// TODO: will this need to be async?
// TODO: how will we separate between these two?
//
//   - transaction verification (i.e. simulation)
//   - transaction ingestion (i.e. destructive execution)
pub fn execute_transaction(transaction: Transaction) -> Result<(), anyhow::Error> {
    // TODO: verify transaction.locktime >= block time

    let Transaction::Version1 {
        inputs, outputs, ..
    } = transaction;

    // TODO: query all UTXOs corresponding to inputs

    for (
        index,
        TransactionInput {
            outpoint,
            signature_script,
        },
    ) in inputs.iter().enumerate()
    {
        dbg!(index, outpoint, signature_script);
        // TODO: lookup input by outpoint.txid and outpoint.index
        // TODO: verify signature script has only data pushes
        // TODO: execute signature script followed by input's pubkey script
        // TODO: accumulate input's value
    }

    for (index, output) in outputs.iter().enumerate() {
        match output {
            TransactionOutput::Value {
                value,
                pubkey_script,
            } => {
                dbg!(index, value, pubkey_script);

                // TODO: if destructive, insert UTXO record
            }

            TransactionOutput::Object {
                object_id,
                data_script,
                pubkey_script,
            } => {
                dbg!(index, object_id, data_script, pubkey_script);

                // TODO: validate data_script

                match object_id.mode {
                    ObjectMode::Fresh => {
                        // TODO: validate object_id.raw == keccak256(inputs.map(|i| i.outpoint), index)
                    }

                    ObjectMode::Existing { permit_index } => {
                        let permit_outpoint = inputs.get(permit_index as usize)
                            .ok_or(anyhow::anyhow!("Index out of bounds: permit_index={permit_index} used by output {index}"))?
                            .clone()
                            .outpoint;

                        dbg!(permit_outpoint);
                        // TODO: lookup UTXO via permit_outpoint
                        // TODO: verify that object_id.raw == permit.object_id.raw
                    }
                }
            }
        }
    }

    Ok(())
}

pub fn compute_fresh_object_id(inputs: Vec<TransactionInput>, index: u32) -> ObjectIdentifier {
    let mut hasher = Keccak256::new();

    for input in inputs {
        hasher.update(input.outpoint.txid);
        hasher.update(bytemuck::cast::<u32, [u8; 4]>(input.outpoint.index));
    }

    hasher.update(bytemuck::cast::<u32, [u8; 4]>(index));

    // TODO: make this safe
    let raw = hasher.finalize().as_slice().try_into().unwrap();

    ObjectIdentifier {
        raw,
        mode: ObjectMode::Fresh,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::tx::engine::execute_transaction;
    use crate::tx::types::{
        Hashable, ObjectIdentifier, ObjectMode, Transaction, TransactionInput, TransactionOutpoint,
        TransactionOutput,
    };
    use anyhow::anyhow;
    use async_trait::async_trait;

    use super::{collect_valid_block_transactions, ExecutionContext};

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
    async fn invalidates_out_of_bounds_permit_index() -> anyhow::Result<()> {
        let inputs: Vec<TransactionInput> = vec![];
        // let object_id = compute_fresh_object_id(inputs.clone(), 0);
        let object_id = ObjectIdentifier {
            raw: [0u8; 32],
            mode: ObjectMode::Existing { permit_index: 0 },
        };

        let result = execute_transaction(Transaction::Version1 {
            inputs,
            outputs: vec![TransactionOutput::Object {
                object_id,
                data_script: vec![],
                pubkey_script: vec![],
            }],
            locktime: 0,
        });

        let err = result.unwrap_err();
        assert_eq!(
            format!("{}", err.root_cause()),
            "Index out of bounds: permit_index=0 used by output 0"
        );

        Ok(())
    }
}
