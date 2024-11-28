---
slug: '/intro'
---

# Intro to Quible

## What is Quible?

Quible is an open-source decentralized certificate authority built with an L1 blockchain and EigenLayer. It allows peer-to-peer networks, which share a common authority, to authenticate with each other using digital signatures and zero-knowledge proofs, thereby simplifying implementations and reducing overhead and latency burdens.

Quible establishes critical public key infrastructure which enables secure low-latency authentication for networks. It reduces the costs normally involved in utilizing digital signatures for permissioned networks and onchain smart contracts.

## Why use a certificate authority?

[JSON Web Token](https://www.rfc-editor.org/rfc/rfc7519) is the industry standard for securing network traffic today. One large disadvantage of using JWTs is the lack of [Public Key Infrastructure](https://en.wikipedia.org/wiki/Public_key_infrastructure), which requires developers to provide their own solution for building, configuring and hosting their token-issuance servers. This can lead to bugs, security vulnerabilities and extra development time.

Traditional certificate authorities do provide the essential PKI that makes TLS/SSL possible, which could be used for securing other systems and peer-to-peer networks. However, many developers will not give serious consideration to them for other non-DNS use cases. This is because they don't provide developer-friendly APIs, they have unpredictable cost metrics and often the X.509 certificate format is too bulky and inconvenient for building applications and servers when compared to JWT.

Sibil attacks are becoming more and more commonplace with every year. One of the most common ways to protect against a Sibil attack is to "lock down" applications and services to a limited set of authorized users/actors via access lists. For large-scale applications, this process can lead to undesirable results when access lists are implemented in naive ways.

## Quible Features Overview

<!-- TODO -->

- Onchain identity management with a UTXO-based blockchain

- High-scale lightweight certificate issuance that can handle extreme loads

- Unique zero-knowledge certificate validation

- EVM bridging for existing NFT-based authentication systems 

- Powered by EigenLayer to reduce risk for users and application developers
