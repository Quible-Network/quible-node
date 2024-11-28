---
sidebar_position: 3
slug: '/rationale'
title: 'Why Quible?'
---

# Why Quible?

Anyone building a permissioned system will face a fundamental problem that can cost them in reduced security, undesirable overhead, bugs, higher operating costs and longer development timelines. Quible provides a streamlined, off-the-shelf solution that delivers improved security, low-latency authentication and more.

### Rationale #1: The impracticality of traditional certificate authorities

Most developers today would pause at the notion of using a certificate authority for authentication. This isn’t because they’re insecure, but instead because the tools available today feel out-of-reach.

Traditional certificate authorities are considered a part of the deeper landscape of web security infrastructure, and due to that nature many developers are never acquainted with technologies such as X.509 certificates, nor the programmatic APIs such as AWS Certificate Manager. At the end of the day, the developer experience involved gives most developers the impression of a bulky, out-of-place and overly-specialized toolset, especially when compared to JSON Web Tokens, an industry standard solution.

To speak about this in very concrete terms, let’s start by mentioning that certificate authorities often charge customers for issuing certificates, which often makes them financially impractical. Additionally, certificate authorities employ an overly-specific design that locks in developers to using them only for DNS-based use-cases. When they are used, still they are designed to have infrequent issuance, and long lifespans, which is polar to the core features of JSON Web Tokens.

When infrastructure teams need certificates to accomodate network SSL needs, they end up resorting to self-signed certificates and internal certificate management because a certificate authority is too heavy weight for their needs.

We believe that Quible doesn’t suffer from this impracticality because it is designed from the start as a developer-focused security tool. We recreate the simplicity and ease-of-use of JSON Web Tokens, while also backing them up with the security advantages of a certificate authority.

### Rationale #2: The hidden cost of supporting JSON Web Tokens

While there are a variety of different ways to employ JSON Web Tokens in an application, they will all run up against some similar constraints which are due to the lack of Public Key Infrastrucutre (PKI). Without this critical infrastructure, JSON Web Tokens necessitate developers build their own solutions for storing, updating and managing their signing keys as well as the solution for issuing the JWTs themselves.

Let’s look at an example. A common situation for an application developer is needing to take an existing API, and open it up so that another business is ready to securely integrate with it. Until this point, their initial use of JSON Web Tokens didn’t have any perceivable pain points because it was trivial for their web server to issue a token to users who have already gone through their session-based signin portal.

Now that the developer needs to open up the access for their API, they will have quite a few decisions to make. How will the other business perform the initial handshake to obtain a token? How we will refresh a token after it expires? Should we use OAuth 2? Should we use Refresh Tokens? What about Rotating Refresh Tokens? How will we document this so that the other business clearly understands what they need to do? This is also not to mention the time it takes for the developer to implement any of these solutions.

Let’s say, hypothetically, that the backend grows into multiple services and the business scales up to large amounts of traffic. New problems arise because the developer is now responsible for configuring the same signing key across multiple services. When an attacker obtains a high-risk token, how do we change the key in real-time without disrupting traffic? We used an environment variable to configure the signing key, so will we have to restart all of our servers at the same time? How do we do that without introducing downtime? Our auth server is under too much load— should we increase our token lifespans? If an attacker secretly compromises your auth signing key, they can start discretely issuing tokens and attacking your system with a low-level of observability.

A lot of problems are starting to build up, not to mention the problem of having enough time to solve these problems.

Teams choose to build in-house JWT tools, or at least host them internally, because introducing a third-party external service for auth will create higher latency characteristics as well as general coupling and concerns for stability/reliability.

The Quible team has seen these situations unfold first-hand in many ways, even at Fortune-500 enterprise companies. Quible was built with this problem-space in mind, and powerfully side-steps almost all of the problems mentioned.

Here are just some mentionable benefits that address these pain points:

- Quible streamlines the authentication process and gives you batteries-included state-of-the-art tooling for this problem space. We have a solution for handshakes, identity management, certificate renewal and more. No more head-scratches.

- Quible is ready-to-use infrastructure. Now you don’t need to worry about scaling your auth server, configuring your services with signing keys or even building an auth server to begin with! Unlike issuing JWTs in-house, here is no incentive to increase your token lifespans. In fact, they can be as short as you want them to be!

- Quible is decentralized and eliminates the problem of having a single-point-of-failure in your auth infrastructure.

- If an attacker does compromise your signing key (the wallet you will use to spend QBL to manage identities), they have a much more limited attack vector than they would with a JWT. Let’s look at what this means:

  - An attacker with your wallet’s signing key will have to add new claims to your identity before they can issue a certificate to themselves. This process happens **out in the open** on the blockchain, which makes it easier to detect an attack and see what they’re doing.

  - Quible makes it super simple to enforce multisig when transferring the ownership of an identity. Even if an attacker is able to add claims to your identities, they would not be able to permanently take down the identity without access to the full set of multisig keys. Your security response team can safely use the multisig to transfer ownership over to new keys which effectively removes the attacker’s access to your auth system.

  - After recovering from an attack, the entire history of identity changes is publically available. This makes it trivial for the security response team to revert the identity configuration to the state from before the attack, effectively returning everything to normal even if the attacker attempted to block your users from authenticating.

  - After all of this, the underlying identities that you’ve published will remain unchanged. This makes for a smaller burden on the operations involved. No need to re-configure your services with a new identity object ID. Everything can go back to business-as-usual.

### Rationale #3: The limitations of existing solutions

When surveying the existing landscape of authentication and attestation solutions available in Web3, we found some awesome tools including [Lit Protocol](https://www.litprotocol.com/) and [Clique](https://www.clique.tech/), not to mention the new market of *zkTLS* products which are all pretty cool.

After giving these options heavy consideration, we found ourselves running up against several undesirable limitations. Let’s briefly talk about them.

- Lit Protocol features [programmable keypairs](https://developer.litprotocol.com/user-wallets/pkps/overview) which effectively allows you to leverage the protocol for issuing signatures on-demand. Here are some of reasons we couldn’t use Lit Protocol to solve our customers’ problems.

  - Getting set up with Lit Protocol requires hand-writing bulky one-time scripts and executing through some trial-and-error. There is a lot of new information to process before you’re able to really get started with the essential functionality.

  - Part of the startup cost is filling a wallet with "Capacity Credits" which are effectively a fee payment. This means you need to pay for each usage of the keypair.

    - In addition to being a literal _cost_ and creating a cumbersome developer experience, we also realized that this creates a huge and easily exploitable attack vector for most applications that build on top of Lit Protocol. If an attacker is able to simply access your application and begin the authentication process, they can simply repeat the authentcation process ad-infinum, until your capacity credits are all used up. This is equivalent to having a system where the user can just repeatedly press the F5 key and eventually take your auth system (and all your users) offline.

- Clique, while very powerful, felt like bringing a nuclear power plant to a drag race.

  - For users who simply want identity-based attestations, the Clique docs are very overwhelming with incredibly heavy-weight examples for verifiable off-chain compute.

  - The bulk of Clique’s feature set is for facilitating off-chain information, similar to zkTLS. While it is still possible to build a data connector for onchain information, this requires a lot of learning, troubleshooting and manual setup process.

  - Similar to Lit Protocol, due to the heavyweight nature, for on-demand real-time application authentication, an attacker can attack your system by repeatedly requesting to perform authentication and you, the application owner, are footing the bill for the costly computations.

Given this context, we wanted to build an auth tool that is not only DoS-resistant, but also something that new developers can understand and begin using in a short amount of time. You can start creating identities in Quible with no one-time scripts and no setup costs. When you build with Quible, certificate issuance is completely free. It takes less than 10 minutes to write the necessary code for identity management, and in less than 10 more minutes, you can have all the code you’ll ever need for verifying the certificates.

We also want to provide straightforward and intuitive GUIs within our block explorer that make common utilitarian tasks a breeze. Need to add a new user to your identity? Just click a few buttons. No code needed. Have an NFT on an EVM chain, and you want to begin validating the NFT ownership status off-chain? Just click a few buttons, paste your contract address, and BOOM! You’re good to go.
