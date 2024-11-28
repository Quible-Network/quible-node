---
sidebar_position: 3
slug: '/certificate-issuance'
---

# Certificate Issuance & Validation

Quible leverages a unique approach to generating certificates, optimized to gracefully handle millions of simultaneous Certificate Signing Requests (CSRs). How is this possible? Let’s dive in.

# Zero-Knowledge Certificates

Instead of performing signing for every CSR, Quible batches the signing process into one certificate-per-block, known as the Block Certificate. Then, when a CSR is received by a node, the node issues a Zero-Knowledge Certificate.

Here’s what is included in a Zero-Knowledge Certificate:

- A copy of the latest Block Certificate, produced by Signer-Validator Nodes.

- Requested identity details:

  - Identity object ID

  - Identity Claim value

  - Configured certificate lifespan for the identity

- A Zero-Knowledge Proof of requested identity details

```
type Certificate =
  { block_certificate : BlockCertificate
  , claim :
    { value : Vec<u8>
    , identity_object_id : [u8; 32]
    , lifespan : u64
    }
  , proofs : Vec<[u8; 32]>
  }
```

### Key Advantages

To support scalability, any Validator Node within the network can issue a certificate without participating in any signing process. This is because Validator Nodes will maintain the state of the blockchain via a [Patricia Merkle Tree](https://docs.alchemy.com/docs/patricia-merkle-tries). The tree stores Zero-Knowledge Proofs for each and every identity and identity claim in the blockchain. As long as a node has a copy of signature of the tree’s root hash (contained in the Block Certificate), that node will have everything it needs to produce a valid Zero-Knowledge Certificate for a Requestor.

Because any Validator Node can issue a certificate, and there is no limit to the number of Validator Nodes that can join the network, this enables Quible to scale to extremely to handle an extremely large volume of CSRs, all without requiring payments for usage. The network becomes DoS-resistant.

# Block Certificates

As mentioned above, Block Certificates contain a copy of the root hash of the Patricia Merkle Tree of the entire state of the blockchain. Obtaining the root hash is a straightforward process that any node can achieve, however obtaining a signature of the root hash is more difficult.

In order to produce a _signed_ root hash, after each block is finalized, the Signer-Validator Nodes perform a Multi-Party Computation with Threshold Signing. Once the threshold is reached, the signature is published to the rest of network, including the non-signer nodes. To accomplish this with desirable low-latency characteristics, Quible uses the [2PC-MPC](https://github.com/dwallet-labs/2pc-mpc) algorithm which provides large-scale multiparty ECDSA signing.

The Block Certificate contains the block hash, the state root hash, the block timestamp and the signature from the signer network.

```
type BlockCertificate =
  { 
  , block_hash : [u8; 32]
  , state_root : [u8; 32]
  , timestamp : u64
  , signature : [u8; 65]
  }
```

# Certificate Validation

In order to perform certificate validation, a machine or entity needs to have a copy of the Root Certificate. This certificate is generated only once, and comes pre-installed with the Quible SDK.

The certificate validation process provided by the Quible SDK is lightweight and is performed as follows:

1. Compute the expiration date for the certificate by adding the lifespan duration to the block timestamp.

2. Using the current system time, validate that the expiration date has not already passed.

3. Validate that signer of the Block Certificate’s signature matches the Global Public Key of the Root Certificate.

4. Validate the certificate’s proof against the Block Certificate’s State Root.
