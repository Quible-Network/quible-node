use sha3::{Digest, Keccak256};

use crate::tx::types::{ObjectMode, Transaction, TransactionInput};

use super::types::{ObjectIdentifier, TransactionOutput};

// TODO: will this need to be async?
// TODO: how will we separate between these two?
//
//   - transaction verification (i.e. simulation)
//   - transaction ingestion (i.e. destructive execution)
pub fn execute_transaction(
    transaction: Transaction, // TODO: include other context parameters, such as block timestamp
) -> Result<(), anyhow::Error> {
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
    use crate::tx::engine::execute_transaction;
    use crate::tx::types::{
        ObjectIdentifier, ObjectMode, Transaction, TransactionInput, TransactionOutput,
    };

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
