# Network Components

The Nym platform knits together several privacy technologies, integrating them into a system of cooperating networked nodes in order to provide network-level privacy and metadata anonymity.

The core components are:
* a **mixnet**, which encrypts and mixes Sphinx packet traffic so that it cannot be determined who is communicating with whom. Our mixnet is based on a modified version of the **Loopix TODO LINK** design.
* a privacy enhancing signature scheme called **zk-nym**, based on the Coconut and Ecash schemes. This allows a shift in thinking about resource access control, from an identity-based paradigm based on _who you are_ to a privacy-preserving paradigm based on _right to use_.
* **Sphinx**, a way of transmitting armoured, layer-encrypted information packets which are indistinguishable from each other at a binary level.
* A CosmWasm-enabled blockchain called **Nyx**, the home of the various smart contracts used by the mixnet.


TODO REDO DIAGRAM
![Nym Platform](../images/nym-platform-dark.png)


* **Nyx Blockchain Validators** secure the network with proof-of-stake Sybil defenses, determine which nodes are included within the network, and work together to create Coconut threshold credentials which provide anonymous access to data and resources. They also produce blocks and secure the Nyx Blockchain. Initially, this chain was used only to house the CosmWasm smart contracts keeping track of Nym's network topology, token vesting contracts, and the `NYM` token itself. In recent months, we've decided to expand the role of Nyx and instead expand its role by making it an open smart contract platform for anyone to upload CosmWasm smart contracts to. Validators also provide privacy-enhanced credentials based on the testimony of a set of decentralized, blockchain-based issuing authorities. Nym validators use the [Coconut](https://arxiv.org/abs/1802.07344) [signature scheme](https://en.wikipedia.org/wiki/Digital_signature) to issue credentials. This allows privacy apps to generate anonymous resource claims through decentralised authorities, then use them with Service Providers.
