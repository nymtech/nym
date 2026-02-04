# Network Components

The Nym Network is built from several types of infrastructure working together.

## Nym Nodes

All traffic-routing infrastructure runs on **Nym Nodes**—a unified binary that operates in different modes. This simplifies deployment and enables future dynamic role assignment based on network conditions.

**Entry Gateways** are the user's first point of contact. They accept client connections via WebSocket, verify zk-nym credentials to confirm payment, and store messages for clients that go offline (up to 24 hours). Entry Gateways know the client's IP address but cannot see message contents or final destinations. They will either create tunnels to Exit Gateways (dVPN mode) or forward Sphinx packets to the first layer of Mix Nodes (in Mixnet mode).

**Mix Nodes** form the three mixing layers that provide core privacy. They receive Sphinx packets, remove one encryption layer, verify integrity, apply a random delay, and forward to the next hop. Mix Nodes cannot determine their position in the route and cannot link incoming packets to outgoing packets.

**Exit Gateways** handle traffic leaving the network. They communicate with external internet services on behalf of users and return responses through the network (dVPN and NymVPN mode), or forward Sphinx packets to receipient Nym Clients (SDK Mixnet mode). Like Tor exit nodes, they can see destination addresses but cannot identify the original sender.

## Nyx Blockchain

Nyx is a Cosmos SDK blockchain that provides coordination services. It maintains the topology registry—the list of active nodes and their public keys—eliminating the need for a centralized directory server. It manages NYM token staking and distributes rewards to node operators. It also hosts the CosmWasm smart contracts that coordinate the node rewarding and credential system.

The blockchain is secured by validators using proof-of-stake consensus. Having the topology on-chain prevents the attacks that plague peer-to-peer directory systems.

## Nym API

Nyx validators operate **Nym API** [instances](/apis/nym-api/mainnet) which provide cached blockchain state. A subset of these also form the "Quorum", handling credential issuance—generating the partial blind signatures that form [zk-nyms](/network/cryptography/zk-nym)- and zk-nym validation.

Credential generation relies on threshold cryptography. No single member can issue credentials alone, and the system remains functional even if some members are offline. This distributes trust across multiple independent parties. See the [zk-nym docs](/network/cryptography/zk-nym) for more on this.

## Decentralization properties

The architecture aims to ensure no single point of compromise:
- Entry Gateways know your IP, but not your activity
- Mix Nodes process your packets but can't trace them
- Exit Gateways see destinations but not sources
- Nyx is decentralized via its validator set, and each member of the Quorum generates partial credentials which are unlinkable to anything
