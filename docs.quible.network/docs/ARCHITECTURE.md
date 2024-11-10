---
sidebar_position: 2
slug: /architecture
---

# Architecture Overview

This document covers the essential concepts used in Quible’s core architecture. As discussed in the introduction page, Quible provides the ability for users to configure identities with a set of claims, and subsequently allows for other users to request certificates which attest that a given set of claims have been associated with a given identity.

### Core Concept: The Quible Network

The Quible Network is a multi-tier fleet of servers, also known as nodes, which run the Quible Node software. The Quible Network has two main responsibilities: identity management and certificate issuance. The two tiers of nodes are defined as follows:

- **Validator Node**: A Validator Node, also known as a *Standalone Quible Node*, provides the essential functionality required for block formation and block consensus by performing block validation when new blocks are proposed by designated block proposers. These nodes provide a JSON-RPC API which allows for end users to submit signed transactions which become stored in a mempool until they are executed or invalidated.

- **Validator‑Signer Node**: A Validator‑Signer Node, also known as an *AVS Quible Node*, provides a superset of the functionality of a Validator Node. A Validator‑Signer Node is an [Actively Validated Service (AVS)](https://docs.eigenlayer.xyz/developers/avs-developer-guide) using the [EigenLayer](https://www.eigenlayer.xyz/) ecosystem. In addition to all of the existing responsibilities held by a Validator Node, a Validator‑Signer Node will handle *Certificate Signing Requests* from users and participate in certificate signing using Decentralized Key Generation and Multi-Party Computation.

### Core Concept: Authors

Authors are users who wish the create and configure identities which are stored within Quible’s blockchain. Authors use a 32-byte ECDSA private key to sign transactions, similar to [Ethereum](https://ethereum.org/), which may be executed by sending them to the Quible Network.

Authors create identities by specifying a set of claims, which are arbitrary-length byte vectors. When an identity is created, it receives a unique identifier, similar to a database identifier, which can be used to reference the identity later for purposes such as updating claims or certificate signing requests.

### Core Concept: Requestors

Requestors are users, or machines, who wish to obtain a certificate. A certificate contains a set of identities and corresponding claims as well as an ECDSA signature signed by the Quible Network. A certificate can be trivially verified by any other machine by checking the signature’s public key against the Quible Network’s global public key, which is also known as the *Root Certificate*.

Certificates are obtained by submitting a *Certificate Signing Request* to a Validator‑Signer Node within the Quible Network. A requestor does not need a wallet nor perform any signing in order to submit the request.

## Blockchain Design

The Quible Network implements a proof-of-stake UTXO-based blockchain. Beyond the normal capabilities of a transferable token, Quible transactions can spend tokens to create persistent stateful objects.

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

Let’s look a little deeper into object outputs. The *data script* which is a sequence of opcodes for modifying the state of the object. The *object ID* is the reference point for the object, so that the chain can identify what object is being modified.

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

As detailed above, object IDs are 32-bytes and have two possible modes.

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

## Certificate Issuance

As mentioned above, Validator‑Signer Nodes participate in certificate signing during the certificate issuance process. This process requires nodes to lookup the latest information about an identity and attest to it via digital signing. There are two ways to store identities and identity claims:

- Store identities and identity claims on the Quible Network using Quible’s identity management features (detailed in our SDK).

- Store identities and identity claims as Non-Fungible Tokens on a third-party EVM network such as Ethereum, Arbitrum, Optimism, etc.

  - When using an NFT, the token’s smart contract is the identity, with the smart contract address being the identifier, and the claim is represented by combining individual token identifiers with their corresponding owner’s wallet address.

In order to lookup NFT contract information, Validator‑Signer Nodes are configured with third-party RPC providers for the EVM networks.

## Node Architecture

The unique approach of Quible’s core architecture reduces risk for the application developers that choose to integrate it by utilizing EigenLayer’s AVS architecture. The Quible Network incentivizes Validator‑Signer Node operators by giving their nodes additional rewards during block proposals. Operators are additionally disincentivized from shutting down or behaving maliciously due to the risk of [slashing](https://a16zcrypto.com/posts/article/the-cryptoeconomics-of-slashing/).

A high-level diagram of this is depicted below.

![architecture diagram](/img/architecture-overview-v1.png)

## Appendix: Decentralized Key Generation

To produce a global public key, also referred to as a *Root Certificate*, the network of Validator‑Signer Nodes must perform a one-time process known as Decentralized Key Generation (DKG). After performing DKG, which is a secret-sharing process, each node will have their own secret which is used for the certificate signing, as well as a copy of a global public key which is used for certificate validation.

### How are new Validator‑Signer Nodes added to the network?

The apparent problem is that when new Validator‑Signer Nodes join the network, they have not participated in the prior DKG process, so they will not have a secret, which prevents them from participating in certificate signing. The Quible Team plans to address this by re-running DKG and re-distributing a new global public key during our testnet and alphanet phases of Quible’s release. As a consequence of supporting identifiable abort during Multi-Party Computation, there may only ever be up to 300 Validator‑Signer Nodes for the official 1.0 Quible Network. Due to this limitation, the Quible Team aims to reduce the chances of requiring subsequent DKG by on-boarding as many node operators as possible before the official release.

However, when Quible completes it’s alpha program, and officially launches the Quible Network 1.0 release, there is still room for adding new Validator‑Signer Nodes if necessary, by utilizing certificate authority chaining, whereby the nodes produce a new global public key via DKG, and certify the new key by signing it with the original global public key. This allows the new Validator‑Signer Nodes to obtain a secret and participate in additional DKG processes. After a certificate chain has been produced, copies of the chained certificates are included in all issued certificates, which allow certificate validators to verify the new certificates with the original global public key.

## Appendix: Identifiable Abort

Nodes can attack the network by beginning to participate in a signing round and subsequently failing to finish the round, requiring the process to be restarted. To mitigate this and disincentivize nodes from attacking this way, Quible utilizes a threshold signature scheme with *identifiable abort* which allows the nodes to identify the ones who are preventing a computation from completing. When a malicious node is identified, it undergoes multiple phases of slashing, whereby eventually, after acting maliciously enough times, the node is completely slashed from the network.
