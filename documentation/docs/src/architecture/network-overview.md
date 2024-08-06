# Network Overview

Nym is a privacy platform. It provides strong network-level privacy against sophisticated end-to-end attackers, and anonymous access control using blinded, re-randomizable, decentralized credentials. Our goal is to allow developers to build new applications, or upgrade existing apps, with privacy features unavailable in other systems.

The Nym platform knits together several privacy technologies, integrating them into a system of cooperating networked nodes.

At a high level, our technologies include:

* a **mixnet**, which encrypts and mixes Sphinx packet traffic so that it cannot be determined who is communicating with whom. Our mixnet is based on a modified version of the **Loopix** design.
* a privacy enhancing signature scheme called **Coconut**. Coconut allows a shift in thinking about resource access control, from an identity-based paradigm based on _who you are_ to a privacy-preserving paradigm based on _right to use_.
* **Sphinx**, a way of transmitting armoured, layer-encrypted information packets which are indistinguishable from each other at a binary level.
* the **Nyx** blockchain, a general-purpose CosmWasm-enabled smart contract platform, and the home of the smart contracts which keep track of the mixnet.

The most important thing to note is that these technologies ensure privacy at two different levels of the stack: **network data transmission**, and **transactions**.

Here's an overview diagram of the different types of nodes making up the network:

![Nym Platform](../images/nym-platform-dark.png)

Developers can think of the network as being comprised of **infrastructure nodes** and **clients** for interacting with this infrastructure via **P**rivacy-**e**nhanced **app**lications (PEApps).

## Mixnet Infrastructure
The mixnet - the different pieces of software that your traffic will pass through when using an privacy-enhanced app (PEApp) - is made up of several different types of nodes:

* **Mix Nodes** provide network security for network content _and_ metadata, making it impossible to see who is communicating with who, by performing packet-mixing on traffic travelling through the network.

* **Gateways** act as message storage for clients which may go offline and come back online again, and defend against denial of service attacks. The default gateway implementation included in the Nym platform code holds packets for later retrieval. For many applications (such as simple chat), this is usable out of the box, as it provides a place that potentially offline clients can retrieve packets from. The access token allows clients to pull messages from the gateway node.

* **Services** are applications that communicate with nym clients, listening and sending traffic to the mixnet. This is an umbrella term for a variety of different pieces of code, such as the network requester binary.

* **Nyx Blockchain Validators** secure the network with proof-of-stake Sybil defenses, determine which nodes are included within the network, and work together to create Coconut threshold credentials which provide anonymous access to data and resources. They also produce blocks and secure the Nyx Blockchain. Initially, this chain was used only to house the CosmWasm smart contracts keeping track of Nym's network topology, token vesting contracts, and the `NYM` token itself. In recent months, we've decided to expand the role of Nyx and instead expand its role by making it an open smart contract platform for anyone to upload CosmWasm smart contracts to. Validators also provide privacy-enhanced credentials based on the testimony of a set of decentralized, blockchain-based issuing authorities. Nym validators use the [Coconut](https://arxiv.org/abs/1802.07344) [signature scheme](https://en.wikipedia.org/wiki/Digital_signature) to issue credentials. This allows privacy apps to generate anonymous resource claims through decentralised authorities, then use them with Service Providers.

## Privacy-enhanced applications (PEApps)
PEApps use a Nym client to connect to the network in order to get the available Network Topology for traffic routing, and send/receive packets to other users and services. Clients, in order to send traffic through the mixnet, connect to gateways. Since applications may go online and offline, a client's gateway provides a sort of mailbox where apps can receive their messages.

Nym clients connect to gateways. Messages are automatically piped to connected clients and deleted from the gateway's disk storage. If a client is offline when a message arrives, it will be stored for later retrieval. When the client connects, all messages will be delivered, and deleted from the gateway's disk.

When it starts up, a client registers itself with a gateway, and the gateway returns an access token. The access token plus the gateway's IP can then be used as a form of addressing for delivering packets.

There are two basic kinds of privacy enhanced applications:

* **Client apps** running on mobile or desktop devices. These will typically expose a user interface (UI) to a human user. These might be existing apps such as crypto wallets that communicate with Nym via our SOCKS5 proxy, or entirely new apps.
* **Service Providers**, which will usually run on a server, and take actions on behalf of users without knowing who they are.

Service Providers (SPs) may interact with external systems on behalf of a user. For example, an SP might submit a Bitcoin, Ethereum or Cosmos transaction, proxy a network request, talk to a chat server, or provide anonymous access to a medical system such as a [privacy-friendly coronavirus tracker](https://constructiveproof.com/posts/2020-04-24-coronavirus-tracking-app-privacy/).

There is also a special category of Service Provider, namely SPs that do not visibly interact with any external systems. You might think of these as crypto-utopiapps: they're doing something, but it's not possible from outside to say with any certainty what their function is, or who is interacting with them.

All apps talk with gateways using Sphinx packets and a small set of simple control messages. These messages are sent to gateways over websockets. Each app client has a long-lived relationship with its gateway; Nym defines messages for clients registering and authenticating with gateways, as well as sending encrypted Sphinx packets.
