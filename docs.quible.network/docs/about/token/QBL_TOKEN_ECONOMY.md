---
title: 'Economy & Costs'
slug: '/token-econony'
---

# QBL Economy & Costs

The Quible Network and the Quible SDK are designed to handle fluctuations in the supply of compute power, such as increases and decreases in the number of Validator Nodes in the network. This design is functionally equivalent to the way fee pricing works in all major blockchains including Bitcoin and Ethereum.

The principle behind fee pricing is that Validator Nodes, during their block proposals, are free to choose which transactions that they include in their proposal. Combine this fact with the variable-fee amount in transactions, and you arrive at a system where node operators can choose to only accept transactions that provide a large enough fee to cover the operation costs.

If a user is submitting transactions without providing a large-enough fee, then their transactions will be ignored by the nodes until either:

- The user submits new transactions with higher fee amounts

or

- The operation costs for the operator become low enough to allow for those transactions to be accepted.

# Compounding Advantage

Due to this nature described above, node operators in the Quible Network can continually adjust their fee amounts to values that correspond to their operating costs. The good news here is that it introduces a compounding advantage, whereby the more users submitting transactions to the network, the lower the fees overall will become. This is because the higher transaction volume, regardless of fee pricing, the more tokens can be earned by the operators. As the transaction volume increases, the node operators can safely lower their fee pricing and still meet their operating costs.
