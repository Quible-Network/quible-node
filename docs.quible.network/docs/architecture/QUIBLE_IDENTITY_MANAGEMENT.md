---
sidebar_position: 1
title: 'Quible Identity Management'
slug: '/quible-identity-management'
---

# Quible Identity Management

Quible offers identity management natively within the blockchain, by expanding upon the normal capabilities of a UTXO Model. Beyond enabling a transferable token, Quible transactions can spend tokens to create persistent stateful objects. This is similar to the [Extended UTXO Model](https://docs.cardano.org/about-cardano/learn/eutxo-explainer/) seen in Cardano, as well as [transaction inscriptions](https://docs.ordinals.com/inscriptions.html) seen in Bitcoin.

## Transaction Design

Let’s explore how this is implemented. In Quible, there are two types of UTXOs: *value* outputs and *object* outputs.

```
type TransactionOutput
  = TransactionOutput
    { pubkey_script : Vec TransactionOpCode
    , details : TransactionOutputDetails
    }

type TransactionOutputDetails
  = Value
    { value : u64
    }
  | Object
    { object_id : ObjectIdentifier
    , data_script : Vec TransactionOpCode
    }
```

Value outputs inherit the same behavior seen in Bitcoin, where their value corresponds to an amount of tokens being spent by the transaction. Object outputs do not contain values, and instead contain information for the creation or modification of persistent stateful objects.

Let’s look a little deeper into object outputs. The *data script* is a sequence of opcodes for modifying the state of the object. The *object ID* is the reference point for the object, so that the chain can identify what object is being modified.

```
type ObjectMode
  = Fresh
  | Existing { permit_index : u32 }

type ObjectIdentifier
  = ObjectIdentifier
    { raw : [u8; 32]
    , mode : ObjectMode
    }
```

As detailed above, object IDs are 32 bytes and have two possible modes.

- **Fresh**: The *fresh* object mode is used to signify that this transaction is creating a new object. When using this mode, the object ID must equal the result of hashing of the IDs of the transaction inputs and the index of the current output.

- **Existing**: The *existing* object mode is used to reference an object that was created in a prior block. In order to build a valid transaction, the transaction must spend the prior unspent transaction object output for that object. The *permit index* parameter refers to the index of the transaction input which is spending the prior unspent transaction object output.

### Transaction Opcodes

Quible supports a minimal set of opcodes for the common [Pay-to-PubKey-Hash](https://bitcoinwiki.org/wiki/pay-to-pubkey-hash) script, as well as additional domain-specific instructions which are used in data scripts for operating on objects as unordered sets of byte vectors.

| Operation      | Purpose       | Parameters | Description |
| -------------- | ------------- | ---------- | ----------- |
| PUSH           | Generic       | Vec u8     | A byte vector is pushed onto the stack. |
| DUP            | Generic       |            | The top stack item is duplicated. |
| CHECKSIGVERIFY | PubKey Script |            | The entire transaction’s outputs, inputs, and script are hashed. A signature and a public key are popped from the stack. The signature must be a valid signature for this hash and public key. If it is not valid, the script fails. |
| EQUALVERIFY    | PubKey Script |            | Two byte vectors are popped from the stack and compared. If they are not equal, the script fails. |
| DELETEALL      | Data Script   |            | All members are deleted from the unordered set. |
| DELETE         | Data Script   | Vec u8     | The member equal to the provided byte vector, if it exists, is deleted from the unordered set. |
| INSERT         | Data Script   | Vec u8     | If there is no member equal to the provided byte vector in the unordered set, it is inserted into the unordered set. |
| SETCERTTTL     | Data Script   |            | Pops a byte vector from the stack. The value is interpreted as a little-endian variable-length unsigned integer. The value is stored as the “Certificate Time-To-Live” for the unordered set, which is used to configure an expiration date when certificates are produced by nodes. |

## Example Walkhrough: Creating identities

Below is an example of how a transaction output is used to create an identity from scratch. In this example, we are an authority that wishes to create a new identity and authorize the users "Alice" and "Bob" to use this identity. This is accomplished by including the values `alice` and `bob` as claims on the identity. In a real example, these would be public keys and not simply the strings of their names.

```
transaction_inputs = [...]

creation_transaction_output = TransactionOutput::Object {

  // This object ID will refer to the identity that we are creating.
  // Alice and Bob will use this ID when requesting certificates.
  object_id: ObjectIdentifier {
    mode: ObjectMode::Fresh,
    raw: keccak256(
      // aggregate the outpoint transaction hash and outpoint index for each transaction input
      ...{ (outpoint.txid, outpoint.index) | input <- transaction_inputs, outpoint <- input.outpoint },

      // include the index of the transaction output
      0
    )
  },

  // this is how we add claims to an identity
  data_script: [
    OpCode::Insert { data: "alice" },
    OpCode::Insert { data: "bob" },
    OpCode::SetCertTtl { data: 86400 },
  ],

  pubkey_script: [...]

}

creation_transaction = Transaction {
  inputs: transaction_inputs,
  outputs: [transaction_output]
}
```

## Example Walkthrough: Updating identities

Below is an example of how a transaction output is used to update the claims of an existing identity. In this example, we wish to remove Bob’s claim, and add a new claim for Carol. In order to for the network to permit us to modify an existing identity, we must spend an unspent transaction output that previously referenced the object for our identity. In this case, we will use the transaction output from our previous example as a transaction input.

```
transaction_inputs = [
  TransactionInput {
    // here we reference the transaction output where we our identity was created
    outpoint: TransactionOutpoint {
      // the transaction hash from the prior example
      txid: creation_transaction.hash,
      index: 0
    },

    signature_script: [...]
  },

  ...
]

transaction_output = TransactionOutput::Object {

  object_id: ObjectIdentifier {

    // here we use the Existing mode with a permit index to
    // refer to the transaction input that is spending the
    // prior object output
    mode: ObjectMode::Existing { permit_index: 0 },

    // instead of calculating a new object_id, we use the object ID
    // of the identity that we wish to update
    raw: creation_transaction.outputs[0].object_id

  },

  // use opcodes to specify changes to the identity claims
  data_script: [

    // 1. remove Bob’s claim
    OpCode::Delete { data: "bob" },

    // 2. add Carol’s claim
    OpCode::Insert { data: "carol" },

  ],

  pubkey_script: [...]

}

update_transaction = Transaction {
  inputs: transaction_inputs,
  outputs: [transaction_output]
}
```
