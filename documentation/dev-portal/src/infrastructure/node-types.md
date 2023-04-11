# Node Types

Discover the workings of Nym's privacy-enhancing mixnet infrastructure through the below video, where we break down the different types of nodes and how they each play a crucial role in ensuring secure and anonymous communication.

<iframe width="700" height="400" src="https://www.youtube.com/embed/rnPpEsJS4FM" title="YouTube video player" frameborder="0" allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share" allowfullscreen></iframe>

### Mixnet Infrastructure

There are few types of Nym infrastructure nodes:

#### Mix Nodes
Mix nodes play a critical role in the Nym network by providing enhanced security and privacy to network content and metadata. They are part of the three-layer mixnet that ensures that communication remains anonymous and untraceable. Mix nodes receive `NYM` tokens as compensation for their quality of service, which is measured by the network validators.

Mix nodes anonymously relay encrypted Sphinx packets between each other, adding an extra layer of protection by reordering and delaying the packets before forwarding them to the intended recipient. Additionally, cover traffic is maintained through mix nodes sending Sphinx packets to other mix nodes, making it appear as if there is a constant flow of user messages and further protecting the privacy of legitimate data packets.

With the ability to hide, reorder and add a delay to network traffic, mix nodes make it difficult for attackers to perform time-based correlation attacks and deanonymize users. By consistently delivering high-quality service, mix nodes are rewarded with NYM tokens, reinforcing the integrity of the Nym network.

#### Gateways
Gateways serve as the point of entry for user data into the mixnet, verifying that users have acquired sufficient NYM-based bandwidth credentials before allowing encrypted packets to be forwarded to mixnodes. They are also responsible for safeguarding against denial of service attacks and act as a message storage for users who may go offline.

Gateways receive bandwidth credentials from users, which are periodically redeemed for `NYM` tokens as payment for their services. Users have the flexibility to choose a single gateway, split traffic across multiple gateways, run their own gateways, or a combination of these options.

In addition, gateways also cache messages, functioning as an inbox for users who are offline. By providing secure, reliable access to the mixnet and ensuring that data remains protected, gateways play a crucial role in maintaining the integrity of the Nym network.

#### Validators
Validators are essential to the security and integrity of the Nym network, tasked with several key responsibilities. They utilize proof-of-stake Sybil defense measures to secure the network and determine which nodes are included within it. Through their collaborative efforts, validators create Coconut threshold credentials which provide anonymous access to network data and resources.

Validators also play a critical role in maintaining the Nym Cosmos blockchain, a secure, public ledger that records network-wide information such as node public information and keys, network configuration parameters, CosmWasm smart contracts, and `NYM` and credential transactions.

#### Service Providers
Service Providers are a crucial aspect of the Nym infrastructure that support the application layer of the Nym network. Any application built with Nym will require a Service Provider, which can be created by anyone. Service Providers run a piece of binary code that enables them to handle requests from Nym users or other services, and then make requests to external servers on behalf of the users.

For example, a Service Provider could receive a request to check a mail server and then forward the response to the user. The presence of Service Providers in the Nym network enhances its security and privacy, making it a reliable and robust platform for anonymous communication and data exchange.

### Where do I go from here? 💭

Maybe you would like to concentrate on building a application that uses the mixnet:

* Explore the Tutorials section of the Developer Portal. Our in-depth tutorial on [Building a Simple Service Provider](../tutorials/simple-service-provider.md) give a good understanding of building User Clients and Service Providers in TypeScript, and how to configure Nym Websocket Clients for seamless communication with the mixnet.

* Get started with using the Nym Mixnet quickly and easily by exploring the [Quickstart](../quickstart/overview.md) options, such a NymConnect, proxying traffic through the Nym Socks5 client, or dive into integrating Nym into your existing application with the [Integrations](../integrations/integration-options.md) section.

Or perhaps you a developer that would like to run a infrastructure node such as a Gateway, Mix node or Network Requestor: 
* Check out the [Network Overview](https://nymtech.net/docs/architecture/network-overview.html) docs page. 

* Take a look at our [Node Setup Guide](https://nymtech.net/docs/nodes/setup-guides.html) with our Nym Docs, containing setup guides for setting up you own infrastructure node.

