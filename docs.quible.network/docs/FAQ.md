---
slug: '/faq'
---

:::warning

We just built this iteration of our docs site and we are continually adding more content. This page is very incomplete and it will be updated very soon with many, many more FAQs.

:::

# Frequently Asked Questions

### Why don’t people just use NFTs?

We’ve seen many solutions out the in wild for NFT-based identity management. Employing ad-hoc NFT ownership status verification
results in highly-coupled solution where the blockchain nodes need to connect to an RPC provider for that node, and subsequently make a network call to check the ownership status. This is undesirable because:

- Developers are now "on the hook" for integrating the RPC provider. It can become a single-point-of-failure for their infrastructure.

- Doing this for every interaction and inbound request results in a lot of extra latency for the network.

  - When faced with this problem, we’ve seen teams choose to add JWTs to reduce the latency for subsequent calls. However, this has resulted in many improvised, inconsistent and cross-incompatible implementations of JWTs across their services. They were left with some spaghetti code on their hands and authenticate is still repeated when changing from one service to another.

    - Additionally, [we have a lengthy rationale for the downsides of adopting JWTs](/rationale#rationale-2-the-hidden-cost-of-supporting-json-web-tokens)

- For use cases where new users need to be registered often and programmatically, it can be undesirable to pay the higher transaction fees of EVM-based chains.

### Do entities need to connect to Quible to verify a certificate?

No. An entity only needs access to the [Root Certificate](/key-terms#root-certificate) to perform verification. This comes pre-installed in all instances of the Quible SDK, effectively allowing Zero-Knowledge Certificates to be verified offline, without any round-trips to the network and without connecting to Quible.

### Is the Quible client lightweight?

Yes. Validator Nodes in the Quible Network provide on-demand UTXO indexing capabilities for clients. Clients can construct and submit transactions, submit Certificate Signing Requests (CSRs) and verify certificates all with an extremely small footprint.
