# Project Smoosh - FAQ

> We aim on purpose to make minimal changes to reward scheme and software. We're just 'smooshing' together stuff we already debugged and know works.  
> -- Harry Halpin,  Nym CEO  

<br>
This page refer to the changes which are planned to take place over Q3 and Q4 2023. As this is a transition period in the beginning (Q3 2023) the [Mix Nodes FAQ page](./mixnodes-faq.md) holds more answers to the current setup as project Smoosh refers to the eventual setup. As project Smoosh gets progressively implemented the answers on this page will become to be more relevant to the current state and eventually this FAQ page will be merged with the still relevant parts of the main Mix Nodes FAQ page. 

If any questions are not answered or it's not clear for you in which stage project Smoosh is right now, please reach out in Node Operators [Matrix room](https://matrix.to/#/#operators:nymtech.chat).

## Overview

### What is project Smoosh?

As we shared in our blog post article [*What does it take to build the wolds most powerful VPN*](https://blog.nymtech.net/what-does-it-take-to-build-the-worlds-most-powerful-vpn-d351a76ec4e6), project Smoosh is:  

> A nick-name by CTO Dave Hrycyszyn and Chief Scientist Claudia Diaz for the work they are currently doing to “smoosh” Nym nodes so that the same operator can serve alternately as mix node, gateway or VPN node. This requires careful calibration of the Nym token economics, for example, only nodes with the highest reputation for good quality service will be in the VPN set and have the chance to earn higher rewards.
> By simplifying the components, adding VPN features and supporting new node operators, the aim is to widen the geographical coverage of nodes and have significant redundancy, meaning plenty of operators to be able to meet demand. This requires strong token economic incentives as well as training and support for new node operators.

## Technical Questions

### What are the changes?

Project smoosh will have three steps:

1. Combine the `gateway` and `network-requester` into one binary ✅
2. Create [exit gateway](../legal/exit-gateway.md): Take the gateway binary including network requester combined in \#1 and switch from *allowed.list* to a new [exit policy](https://nymtech.net/.wellknown/network-requester/exit-policy.txt) ✅
3. Combine all the nodes in the Nym Mixnet into one binary, that is `mixnode`, `gateway` (entry and exit) and `network-requester`.

These three steps will be staggered over time - period of several months, and will be implemented one by one with enough time to take in feedback and fix bugs in between.  
Generally, the software will be the same, just instead of multiple binaries, there will be one Nym Mixnet node binary. Delegations will remain on as they are now, per our token economics (staking, saturation etc)

### What does it mean for Nym nodes operators?

We are exploring two potential methods for implementing binary functionality in practice and will provide information in advance. The options are:

1. Make a selection button (command/argument/flag) for operators to choose whether they want their node to provide all or just some of the functions nodes have in the Nym Mixnet. Those nodes functioning as exit gateway (in that epoch) will then have bigger rewards due to their larger risk exposure and overhead work with the setup.

2. All nodes will be required to have the exit gateway functionality. All nodes are rewarded the same as now, and the difference is that a node sometimes (some epochs) may be performing as exit gateway sometimes as mix node or entry gateway adjusted according the network demand by an algorithm.

### Where can I read more about the exit gateway setup?

We created an [entire page](../legal/exit-gateway.md) about the technical and legal questions around exit gateway. 

### What is the change from allow list to deny list?

The operators running `gateways` would have to “open” their nodes to a wider range of online services, in a similar fashion to Tor exit relays. The main change will be to expand the original short allow list to a more permissive setup. An [exit policy](https://nymtech.net/.wellknown/network-requester/exit-policy.txt) will constrain the hosts that the users of the Nym VPN and Mixnet can connect to. This will be done in an effort to protect the operators, as Gateways will act both as SOCKS5 Network Requesters, and exit nodes for IP traffic from Nym VPN and Mixnet clients.

### Can I run a mix node only?

Depends which [design](./smoosh-faq.md#what-does-it-mean-for-nym-nodes-operators) will be the final one. In case of the first one - yes. In case of the second option, all the nodes will be setup with gateway functionality turned on.

## Token Economics & Rewards

### What are the incentives for the node operator?

In the original setup there were no incentives to run a `network-requester`. After the transition all the users will buy multiple tickets of zkNyms credentials and use those as [anonymous e-cash](https://arxiv.org/abs/2303.08221) to pay for their data traffic ([`Nym API`](https://github.com/nymtech/nym/tree/master/nym-api) will do the do cryptographical checks to prevent double-spending). All collected fees get distributed to all active nodes proportionally to their work by the end of each epoch.

### How does this change the token economics?

The token economics will stay the same as they are, same goes for the reward algorithm.

### How are the rewards distributed?

This depends on [design](./smoosh-faq.md#what-does-it-mean-for-nym-nodes-operators) chosen. In case of \#1, it will look like this:

As each operator can choose what roles their nodes provide, the nodes which work as open gateways will have higher rewards because they are the most important to keep up and stable. Besides that the operators of gateways may be exposed to more complication and possible legal risks.

The nodes which are initialized to run as mix nodes and gateways will be chosen to be on top of the active set before the ones working only as a mix node. 

I case we go with \#2, all nodes active in the epoch will be rewarded proportionally according their work. 

### How will be the staking and inflation after project Smoosh?

We must run tests to see how many users pay. We may need to keep inflation on if not enough people pay to keep high quality gateways on in the early stage of the transition.

### When project smooth will be launched, it would be the mixmining pool that will pay for the gateway rewards based on amount of traffic routed ?

Yes, the same pool. Nym's aim is to do minimal modifications. The only real modification on the smart contract side will be to get into top X of 'active set' operators will need to have open gateway function enabled.

### What does this mean for the current delegators?

From an operator standpoint, it shall just be a standard Nym upgrade, a new option to run the gateway software on your node. Delegators should not have to re-delegate.

## Legal Questions

### Are there any legal concerns for the operators?

So far the general line is that running a gateway is not illegal (unless you are in Iran, China, and a few other places) and due to encryption/mixing less risky than running a normal VPN node. For mix nodes, it's very safe as they have "no idea" what packets they are mixing.  

There are several legal questions and analysis to be made for different jurisdictions. To be able to share resources and findings between the operators themselves we created a [Community Legal Forum](../legal/exit-gateway.md). 

