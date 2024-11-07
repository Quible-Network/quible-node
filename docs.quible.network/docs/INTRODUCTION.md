---
sidebar_position: 1
slug: /
---

# Introduction

Quible is a blockchain-powered decentralized certificate authority, designed to fulfill practical needs for authentication in a variety of use cases. Traditional certificate authorities are underutilized today because they are often overlooked and considered to be only appropriate for certifying domain names in the context of TLS/SSL connections. Quible aims to bridge the gap between signature-based authentication standards such as JSON Web Tokens and the lesser-known world of certificate authorities.

Quible operates differently from traditional CAs in several ways. First, Quible stores identities and identity claims within a blockchain, replacing the need for applications and developers to build their own certificate management solution. Second, Quible handles certificate signing requests on-demand, allowing for easy integrations and short certificate lifespans. Similar to JSON Web Tokens, Quible certificates are much simpler and easy to work with than traditional X.509 certificates.

## How does Quible work?

At its core, the Quible Network is a decentralized proof-of-stake blockchain. Nodes within the Quible Network are responsible for minting blocks through normal proof-of-stake block consensus, as well as participating in multi-party computations to issue signed certificates. Nodes within the Quible Network use decentralized key generation to produce a global public key, also known as the root certificate, with which all certificates are signed. All certificates issued by the Quible Network can be verified against the global public key, by anyone, anywhere.

Certificates contain identity data corresponding to the identity data stored by the blockchain. Applications can create and modify identities at any time by sending transactions to the Quible Network. This allows applications to authorize (and reject) new network members in real-time.

<!-- TODO: [diagram here] -->
