# Network Components

The Nym Network is built from several types of infrastructure working together. No single component has enough information to break privacy.

## Nym Nodes

All traffic-routing infrastructure runs on **Nym Nodes**—a unified binary that operates in different modes. This simplifies deployment and enables future dynamic role assignment based on network conditions.

**Entry Gateways** are the user's first point of contact. They accept client connections via WebSocket, verify zk-nym credentials to confirm payment, and store messages for clients that go offline (up to 24 hours). Entry Gateways know the client's IP address but cannot see message contents or final destinations.

**Mix Nodes** form the three mixing layers that provide core privacy. They receive Sphinx packets, remove one encryption layer, verify integrity, apply a random delay, and forward to the next hop. Mix Nodes cannot determine their position in the route and cannot link incoming packets to outgoing packets.

**Exit Gateways** handle traffic leaving the mixnet. They communicate with external internet services on behalf of users and return responses through the network. Like Tor exit nodes, they can see destination addresses but cannot identify the original sender.

## Nyx Blockchain

Nyx is a Cosmos SDK blockchain that provides coordination services. It maintains the topology registry—the list of active nodes and their public keys—eliminating the need for a centralized directory server. It manages NYM token staking and distributes rewards to node operators. And it hosts the smart contracts that coordinate the credential system.

The blockchain is secured by validators using proof-of-stake consensus. Having the topology on-chain prevents the attacks that plague peer-to-peer directory systems.

## Nym API

A subset of Nyx validators operate **Nym API** instances, forming the "Quorum." This group performs network monitoring by sending test packets through the mixnet and calculating reliability scores for nodes. More critically, the Quorum handles credential issuance—generating the partial blind signatures that form zk-nyms.

The Quorum uses threshold cryptography. No single member can issue credentials alone, and the system remains functional even if some members are offline. This distributes trust across multiple independent parties.

## Decentralization properties

The architecture ensures no single point of compromise. Entry Gateways know your IP but not your activity. Mix Nodes process your packets but can't trace them. Exit Gateways see destinations but not sources. The blockchain is decentralized via its validator set. The Nym API Quorum requires threshold agreement.

Even if some nodes are malicious, privacy holds as long as at least one honest node exists on each route. Route selection is random and independent per-packet, making it infeasible to predict or manipulate paths.

## Scale

The current deployment includes over 600 active nodes across approximately 60 countries, operated by independent parties worldwide. For information on running infrastructure, see the [Operator Documentation](/operators).
