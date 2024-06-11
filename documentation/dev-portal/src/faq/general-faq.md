# Nym General FAQ

### What is Nym? 

Nym is a privacy platform that secures user data and protects against surveillance at the network level. 

The platform does so by  leveraging different technological components:  

- **Nym Mixnet**, a type of overlay network that makes both content and metadata of transactions private through mixing, network-level obfuscation and incentivisation (using Sphinx);
- A blockchain called **Nyx**, our Cosmos SDK blockchain, to allow for us to use payment tokens in the form of NYM, as well as smart contracts, in order to create a robust, decentralized, and secure environment incentives for the Mixnet;
- **Coconut**, a zero-knowledge signature scheme, that creates an application-level private access control layer to power Zk-Nyms;
- A utility token **NYM**, to pay for usage, measure reputation and serve as rewards for the privacy infrastructure.

Simply put, the [Nym network ("Nym")](https://www.feat-nym-update-nym-web.websites.dev.nymte.ch/nym-whitepaper.pdf) is a decentralized and incentivized infrastructure to provision privacy to a broad range of message-based applications and services. Think of it as a [*"Layer 0" privacy infrastructure*](https://blog.nymtech.net/nym-layer-0-privacy-infrastructure-for-the-whole-internet-e53238f9b8e7) for the entire internet.

**Related articles:**  
- [*Nym is not a blockchain, but is powered by one*](https://blog.nymtech.net/nym-is-not-a-blockchain-but-it-is-powered-by-one-4bb16ef16587)
- [*Nym tokens*](https://blog.nymtech.net/nym-tokens-where-do-they-live-and-how-are-they-distributed-cross-chain-8d134bf9c41f)
- [*A breakdown of the smart contracts that run the Nym token economics*](https://blog.nymtech.net/a-breakdown-of-the-smart-contracts-that-run-the-nym-token-economics-3a61b4139f95)
- [*Zk-Nyms are here*](https://blog.nymtech.net/a-breakdown-of-the-smart-contracts-that-run-the-nym-token-economics-3a61b4139f95)
- [*Sphinx: the anonymous data format that powers Nym*](https://blog.nymtech.net/sphinx-tl-dr-the-data-packet-that-can-anonymize-bitcoin-and-the-internet-18d152c6e4dc)


### What's the difference between Nym and VPNs?

Nym is not an onion routing system, it is not a decentralized VPN - it’s much more than that. Nym is a mixnet meant to stop precisely the traffic analysis attacks that Tor and dVPNs are vulnerable to. 

It is an orthogonal design that maintains better privacy and can support anonymity, although usually with a cost in terms of latency.
It basically is an infrastructure on which privacy preserving apps can be built, leveraging the Mixnet and Coconut credentials, amongst others. 
The **Nym mixnet and VPNs differ** because VPNs do not mix nor do they protect metadata from an adversary who may be able to watch the entire network. 

**Related articles:**   
- [*VPNs, Tor, I2P, how does Nym compare? *](https://blog.nymtech.net/vpns-tor-i2p-how-does-nym-compare-8576824617b8)


### What is Nym’s VPN?

Since Q2 2023 the Nym core team has been working on launching the **first major consumer facing product** that runs on top of the Nym mixnet: a high speed, trustless and decentralized VPN, paid for via the NYM token - facilitating anonymous payments if wished. 
The product positions itself as a full-network protection service available across all of a user’s devices, **leveraging the Nym Mixnet** and other primitives to offer split tunneling and traffic obfuscation techniques to protect against censorship. 

**Related articles:**  
- [*What does it take to build the most powerful VPN?*](https://blog.nymtech.net/what-does-it-take-to-build-the-worlds-most-powerful-vpn-d351a76ec4e6)

