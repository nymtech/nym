# Nym vs Other Systems

The diagram and brief explainer texts below give a high level overview of the difference between Nym and other comparable systems.

> If you want to dig more deeply into the way traffic is packetised and moved through the mixnet, check out the [Mixnet Traffic Flow](https://nymtech.net/docs/architecture/traffic-flow.html) page of the docs.
<!-- IMG DOES NOT EXIST
![](../images/nym-vs-animation.gif)
-->

### Nym vs VPNs

The most popular network-level privacy solution currently is the VPN (virtual private network), which provides network-level protection via an encrypted tunnel between a user’s computer and one run by a VPN provider. VPNs are often misconfigured, however, and even when configured correctly, don’t offer real privacy or adequate resistance to censorship.

VPN providers can also fully observe all network traffic between users and the public internet, knowing exactly what services its users are accessing at a given time. The user must trust that the VPN provider is not using their information in a malicious manner or keeping logs.

The Nym mixnet is an anonymous overlay network that provides strong network-level anonymity, even in the face of powerful systems capable of passively monitoring the entire network. The mixnet is decentralized, with no trusted third parties, and so does not require a trusted provider like a VPN. More importantly, Nym provides superior privacy to VPNs and can support high-quality of service and low latency through incentives.

### Nym vs Tor

Tor is the best-known anonymous overlay network today. Unlike VPNs, Tor provides a ‘circuit’ of three hops that provides better privacy than single-node VPNs, so any single node in Tor can’t deanonymize traffic. Tor’s onion-routing encrypts traffic between each hop so that only the final hop, the Tor ‘exit node’, can decrypt the package.

However, Tor’s anonymity properties can be defeated by an entity that is capable of monitoring the entire network’s ‘entry’ and ‘exit’ nodes, because while onion-routing encrypts traffic, Tor does not add timing obfuscation or use decoy traffic to obfuscate the traffic patterns which can be used to deanonymize users. Although these kinds of attacks were thought to be unrealistic when Tor was invented, in the era of powerful government agencies and private companies, these kinds of attacks are a real threat. Tor’s design is also based on a centralized directory authority for routing.

While Tor may be the best existing solution for general-purpose web-browsing that accesses the entire internet, it is inarguable that mixnets are better than Tor for message-passing systems such as cryptocurrency transactions and secure messaging, and we believe well designed incentives can also enable the use of Nym as a general purpose decentralized VPN. The Nym mixnet provides superior privacy by making packets indistinguishable from each other, adding cover traffic, and providing timing obfuscation. Unlike both previous mixnet designs and Tor, the Nym mixnet decentralizes its shared operations using blockchain technology and uses incentives to both scale and provide censorship-resistance.

### Nym vs I2P

I2P (‘Invisible Internet Project’) replaces Tor’s directory authority with a distributed hash table for routing. How to design a secure and private distributed hash table is still an open research question, and I2P is open to a number of attacks that isolate, misdirect, or deanonymize users. Like Tor, I2P is based on ‘security by obscurity’, where it is assumed that no adversary can watch the entire network. While security by obscurity may have been cutting-edge at the turn of the millennium, such an approach is rapidly showing its age.

Nym’s cutting-edge mixnet design guarantees network anonymity and resistance to surveillance even in the face of powerful deanonymizing attacks. Unlike I2P, Nym adds decoy traffic and timing obfuscation. Rather than a centralized directory authority or distributed hash table, Nym uses blockchain technology and economic incentives to decentralize its network.The Nym mixnet can anonymize metadata even against government agencies or private companies who can monitor network links and observe the incoming and outgoing traffic of all clients and servers.

### Nym vs Facebook Connect

The Nym credential system decentralizes the functions of systems like Facebook Connect while adding privacy. Personal data has become a toxic asset, even to companies who base their entire business around it, as evidenced by the hack of Facebook’s OAuth identity system in 2018 and the subsequent release of the data of 50 million users.

Unlike Facebook Connect and similar OAuth-based services like Sign in with Google, traditional usernames and passwords, or even public/private key pairs, Nym credentials allow users to authenticate and authorize data sharing without unwillingly revealing any information to a third party. There is no central third party in charge of the credentials, and users remain totally in control of their own data, disclosing it only to those who they want to. A user can store their data wherever they want (including on their own devices), and unlike alternatives like W3C’s DIDs, a user does not store anything on the blockchain, offering better privacy.
