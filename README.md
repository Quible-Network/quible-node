Quible Node
===========

*Flagship reference implementation of the Quible Protocol, for use in the [Quible Network](https://quible.network).*

### What is Quible?

Quible is a proof-of-stake blockchain, designed and built from scratch to facilitate decentralized ECDSA signature generation via multi-party computation. At its core, it offers the ability for a user to upload an unordered set of identifiers, and subsequently request signed proofs which indicate whether a given identifier is present (or not present) in the set. This feature is utilized for implementing access control, by allowing users to upload their access list and generate EVM-friendly ECDSA signatures on-demand from the Quible Network.

![quible diagram](docs/quible-diagram.svg)

## Installation

*Installation instructions coming soon!*

## Acknowledgements

Big thanks to:

- [The Quible Team](https://quible.network) for building this implementation
- [dWallet Labs](https://pera.io) for inspiring the project with the innovation of [2PC-MPC ECDSA](https://github.com/dwallet-labs/2pc-mpc).
- [Paradigm](https://www.paradigm.xyz/) for building [reth](https://github.com/paradigmxyz/reth), providing a good reference for building state-of-the-art Proof-of-Stake in Rust.
