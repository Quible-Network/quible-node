---
sidebar_position: 4
slug: '/use-cases'
title: 'Use Cases'
---

# Use Cases

Below is a brief overview of some of our use cases. Many application developers, blockchain developers and protocol developers often face the need to manage and verify unique identities. The common theme seen here is that they are choosing to manage their identities by issuing a Non-Fungible Token (NFT) on a third-party chain, and building their infrastructure in order to perform the required identity verification process.

## Node License Management & Verification

Node license sales are on the rise in Web3. A common challenge introduced by improvised license management is license verification. As mentioned above, the most popular solution for managing licenses is to issue an NFT on a third-party chain. This results in highly-coupled solution where the blockchain nodes need to connect to an RPC provider for that node, and subsequently make a network call to check the ownership status of the NFT.

First, Quible addresses this use case by providing EVM bridging solutions which allow these teams to continue using their NFT-based identity management. Second, Quible provides the necessary infrastructure to make identity verification incredibly trivial to implement, when compared to the cost of building the ad-hoc NFT ownership status verification as mentioned above.

## First-Party Peer Authorization Management

When building a protocol, oftentimes there is a need for the development team to authorize users instead of allowing permissionless interactions with the protocol. This protects against Sibil attacks, as well as maintains accountability for legal compliance. This is especially useful for teams and projects who are in an early phase of development, and do not wish to open up access to the public before their official launch.

For these projects, Quible makes it easy and lightweight to integrate this authorization system, whether they want to use NFTs or manage their identities on Quible directly, by streamlining the process with a clean SDK and integration guide.

## Third-Party Peer Authorization Management

When operating in a peer-to-peer network, such as an IoT network, it can be desirable to authenticate machine-based entities against their manufacturer. With Quible, manufacturers can publish their official (obfuscated) serial/IMEI numbers on Quible.

With this ability, public services can prevent unknown machines from interacting, or build DoS protection by granting bypass permissions and traffic priority to trusted peers without completely rejecting requests from permissionless users.

## Connected Cars

Autonomous vehicles require robust communication infrastructure to ensure safe operation in the real world. Broadly speaking, there are two types of communication such infrastructure should support: centralized and peer-to-peer. Centralized communication occurs with a private server to perform data logging, authentication, and other services which are orchestrated by the host company. Peer-to-peer communication occurs between vehicles on the road.

Two current pain points for peer-to-peer vehicle communication include manufacturer-specific certificates and offline environments. Manufacturers each issue their own proof of identity for autonomous vehicles in their networks that allow them to authenticate with the centralized server, but cross-manufacturer authenticated communication cannot happen without the presence of a third-party certificate or mutual verification mechanism.

Additionally, communication with this centralized server (on which the aforementioned verification mechanism would be hosted, if it exists), requires access to an internet connection. Quible solves both of these problems by acting as a third-party certificate authority that can provide proofs of identity on-demand and offline. Our decentralized network also delivers latency significantly better than the status quo for inter-vehicle communication.

## Agentic Networks

With the rise of AI agents, the market is seeing an increasing number of companies orchestrating large (100K+) fleets of servers that host large language model instances to perform some set of tasks or experiments. Managing these large server networks is a non-trivial task that currently involves a complex web of SSH key management and JSON Web Token issuance and rotation.

Both of these processes are highly error-prone because of the degree to which they require developers or server administrators to manually manage individual identities. Quible abstracts away the cumbersome task of issuing and verifying identities to individual agent hosts by attaching identities to public keys and supporting certificate verification at every layer of its blockchain network. This enables us to deliver highly secure authentication to AI agent fleets with state-of-the-art latency.


## Onchain Event Participation

Whether it’s an NFT sale, an airdrop, an ICO token sale or RWA issuance, there are many forms of permissioned onchain events. These typically use merkle trees to manage their participant lists. One of the several drawbacks behind this approach includes the requirement to build infrastructure for maintaining the merkle tree and gracefully handling updates. If you are registering new users in real-time, it requires you, the event owner, to pay for transaction fees for each newly registered user. There are no easy ways to give other parties the authority to register new users. Ultimately, the event owner has to provide their own solutions for access management.

To look further at some examples of this in the wild, we can see that [Arbitrum](https://web.archive.org/web/20241128004330/https://www.clique.tech/cases/arb), [Optimism](https://web.archive.org/web/20241128004347/https://www.clique.tech/cases/op), [Mantle](https://web.archive.org/web/20241128004406/https://www.clique.tech/cases/mantle) and [Ronin](https://web.archive.org/web/20241128004422/https://www.clique.tech/cases/ronin) have all leaned on a decentralized attestation mechanism to assist in running onchain events with offchain identity components. Quible provides a Solidity SDK which makes it simple and easy for event owners to employ a Quible-based access list, while leveraging all of the features of Quible itself.

## Throttle Bypass

For protocols that wish to disincentivize processes such as unstaking, a common employed technique is throttling user actions such as delaying their action and limiting the total amount of tokens that can change hands in a given span of time. Oftentimes with these protocols, there are entites which are trusted by the protocol and are considered to be very important as well. To provide a better experience for these VIP-level users, protocols provide a bypass mechanism whereby these selected users can opt to forego the usual penalties of the protocol.

Quible provides tailored tools for implementing this bypass mechanism with a Solidity SDK, and simple-to-use identity management functionality.
