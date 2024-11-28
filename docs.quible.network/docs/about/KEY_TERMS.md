---
slug: '/key-terms'
sidebar_position: 2
---

# Key Terms

- **Author/Authority**: An entity that is trusted by a network or an application to authorize a set of a values, usually public keys, to be permitted to use a permissioned application or connect to a permissioned network. This is also known as a *Registration Authority*.

- **Certificate Authority**: A trusted organization or entity that is responsible for issuing and managing digital certificates used to secure online communications. Quible is a Certificate Authority that issues certificates on behalf of other Authorities which have published their identity configuration to the Quible Network.

- **Certificate Signing Request (CSR)**: A payload or excerpt that an entity submits to a Certificate Authority when requesting a digital certificate. The contents of the request must be validated by the Certificate Authority before the certificate is issued. The contents will usually include an identity and the identity claim which corresponds to the entity’s public key.

- **Certificate Validator**: An application or an entity within a network, that verifies the authenticity and validity of a digital certificate. This ensures that the certificate can be trusted for securing communications, identifying entities or authorizing operations.

- **Decentralized Key Generation (DKG)**: A cryptographic protocol used in systems where multiple participants collaboratively generate a shared key without requiring a trusted central Authority. This process is particularly important in decentralized systems to enhance security, reliability, and trustlessness.

  - Quible uses DKG to produce a one-time signing key and verifying key which is subsequently used during all MPC signing operations performed by the network.

- **Decentralized Bridging**: A system that facilitates the transfer of data between different blockchains without a central authority. Quible uses decentralized bridging to allow Authors and Requestors to store their identity configuration on other chains, usually EVM chains, and still retain the ability to use Quible’s core features.

- <span id="root-certificate">**Digital Certificate**</span>: An electronic file or excerpt that confirms the authenticity of the entity to which they are issued to, which establishes trust in the digital world. Quible’s core architecture utilizes three types of certificates:

  - (internal) **Root Certificate**: A copy of the global public key that originated during the Decentralized Key Generation (DKG) process performed by Signer-Validator Nodes. The Root Certificate is pre-installed in Quible’s SDK, which allows certificate validation to happen instantly, without a round-trip to the network.

  - (internal) **Block Certificate**: A snippet or payload that includes the State Root from the latest block and a digital signature that can be validated against the Root Certificate. Signer-Validator Node participate in Multi-Party Computation and Threshold Signing every time a block is finalized.

  - (external) **Zero-Knowledge Certificate**: A snippet or payload that includes a Block Certificate and a Zero-Knowledge Proof that can be validated against the State Root contained in the Block Certificate. The Zero-Knowledge Proof and corresponds to a leaf of the State Tree which contains an identity, an identity claim and a configured lifespan duration.

    - A Certificate Validator can validate a Zero-Knowledge Certificate by first validating the signature of the Block Certificate against the Root Certificate and subsequently validating the Zero-Knowledge Proof against the State Root contains in the Block Certificate.


- **EigenLayer**: A protocol and ecosystem built on Ethereum that enables the operation of Actively Validated Services (AVSs). An AVS provides added security for the end-users by strongly disincentivizing misbehavior and instability.

- **Identity Management**: A system or process where an entity, acting as an Author or an Authority, publishes information which becomes canonically associated with the identity of that entity. This usually involves an Authority publishing a set of public keys within a list in order to "authorize" them to be used with other applications or networks.

- **Identity Claim**: A value, usually a public key, that has been associated with an identity. If a user has their public key added as a claim to an identity, this means that the user "has a claim" to that identity and can authentically claim to be authorized to use it.

- **Identity**: A unique entity that can be created by an author, which is trusted by applications or networks. An identity can be owned by any number of authors, and ownership can be transferred.

- **L1 Blockchain**: The foundational layer of a blockchain network, serving as the base protocol where transactions are processed and recorded. All operations take place directly within the network without reliance on external solutions.

- **Multi-Party Computation (MPC) & Threshold Signing**: A cryptographic technique that allows multiple parties to collaborate on signing a message without any single party possessing the private key. Specifically when used with Threshold Cryptography, it allows for collaborative signing when only a subset of the parties are participating.

- **Node**: A participant, usually a machine or server, in a blockchain network that maintains and validate’s the blockchain’s core operations. The Quible Network is comprised of two tiers of nodes:

    - **Validator Node**: A node which performs essential actions block validation, block proposals and certificate issuance. Validator nodes are important because they can be run outside of EigenLayer, and there can be any number of validator nodes in the network.

    - **Signer-Validator Node**: A node which holds a superset of responsibilities of a Validator Node. These additional responsibilities comprise of participating in Decentralized Key Generation (DKG), Multi-Party Computation (MPC), Threshold Signing. In order to facilitate low-latency signing, only up to 300 Signer-Validator Nodes can participate in the network. Due to this restriction, and the higher security responsibility, Signar-Validator Nodes must be run on EigenLayer and will be subject to more severe penalties for instability and malicious behavior.

- **Proof-of-Stake**: A consensus mechanism used in blockchain networks to achieve agreement on the state of the blockchain while maintaining security, decentralization, and scalability. Unlike Proof-of-Work (PoW), which relies on computational power, PoS uses the stake (ownership or holdings) of cryptocurrency tokens to determine a participant’s ability to validate transactions and create new blocks.

- **Public Key Infrastrucutre**: A framework of technologies, policies and procedures used to manage digital certificates and public-key encryption. This includes the certificate model, the trust model, the certificate authorities and the registration authorities (we refer to these as authors or authorities).

- **Requestor**: An application or entity that submits Certificate Signing Requests to a Certificate Authority in order to receive a certificate. After receiving a certificate, the Requestor will use this certificate when using a permissioned application or connecting to a permissioned network.

- **State Root**: A cryptographic hash that represents the entire state of a blockchain at a specific point in time. The state root ensures data integrity and enables efficient verification of the blockchain’s current state. In Quible, this hash is computed by building a Merkle Patricia Tree containing the state of all identity and identity claims in the blockchain, and deriving the hash root, also known as the Merkle Root.

- **Unspent Transaction Output (UTxO) Model**: A way a organizing and managing transactions in blockchain systems. Instead of tracking the balance of an account, as in an account-based model used by EVM, the UTXO Model focuses on individual transaction outputs and whether they have been spent or not.

- **Zero-Knowledge Proof**: A cryptographic method that allows one party (the prover) to prove to another party (the verifier) that they know a piece of information (e.g., a secret or a solution) without revealing the actual information itself.

    - These are used by Quible’s Zero-Knowledge Certificates in order to drastically reduce the overhead involved in issuing a certificate with a digital signature. Because of the zero-knowledge design, the Quible Network only needs to generate one Digital Signatuer per block, and relies on Zero-Knowledge Proofs to prove authenticity without needing to reveal the entire state of the blockchain.
