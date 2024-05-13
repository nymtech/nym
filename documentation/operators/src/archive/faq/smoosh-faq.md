# Project Smoosh - FAQ

```admonish warning
**This is an archived page for backwards compatibility. We have switched to [`nym-node` binary](../../nodes/nym-node.md), please [migrate](../../nodes/setup.md#migrate) your nodes. The content of this page is not updated since April 26th 2024. Eventually this page will be terminated!**
```

> We aim on purpose to make minimal changes to reward scheme and software. We're just 'smooshing' together stuff we already debugged and know works.
> -- Harry Halpin,  Nym CEO

<br>

This page refer to the changes which are planned to take place over Q3 and Q4 2023. As this is a transition period in the beginning (Q3 2023) the [Mix Nodes FAQ page](mixnodes-faq.md) holds more answers to the current setup as project Smoosh refers to the eventual setup. As project Smoosh gets progressively implemented the answers on this page will become to be more relevant to the current state and eventually this FAQ page will be merged with the still relevant parts of the main Mix Nodes FAQ page.

If any questions are not answered or it's not clear for you in which stage project Smoosh is right now, please reach out in Node Operators [Matrix room](https://matrix.to/#/#operators:nymtech.chat).


### What are the changes?

Project Smoosh will have four steps, please follow the table below to track the dynamic progress:

| **Step**                                                                                                                                                                                                                                                                                                                                         | **Status**     |
| :---                                                                                                                                                                                                                                                                                                                                             | :---           |
| **1.** Combine the `nym-gateway` and `nym-network-requester` into one binary                                                                                                                                                                                                                                                                     | ✅ done        |
| **2.** Create [Exit Gateway](../../legal/exit-gateway.md): Take the `nym-gateway` binary including `nym-network-requester` combined in \#1 and switch from [`allowed.list`](https://nymtech.net/.wellknown/network-requester/standard-allowed-list.txt) to a new [exit policy](https://nymtech.net/.wellknown/network-requester/exit-policy.txt) | ✅ done        |
| **3.** Combine all the nodes in the Nym Mixnet into one binary, that is `nym-mixnode`, `nym-gateway` (entry and exit) and `nym-network-requester`.                                                                                                                                                                                               | ✅ done        |
| **4.** Adjust reward scheme to incentivise and reward Exit Gateways as a part of `nym-node` binary, implementing [zkNym credentials](https://youtu.be/nLmdsZ1BsQg?t=1717).                                                                                                                                                                       | 🛠️ in progress |
| **5.** Implement multiple node functionalities into one `nym-node` connected to one Nyx account.                                                                                                                                                                                                                                                 | 🛠️ in progress |

These steps will be staggered over time - period of several months, and will be implemented one by one with enough time to take in feedback and fix bugs in between.
Generally, the software will be the same, just instead of multiple binaries, there will be one Nym Node (`nym-node`) binary. Delegations will remain on as they are now, per our token economics (staking, saturation etc)

### What does it mean for Nym nodes operators?

We are exploring two potential methods for implementing binary functionality in practice and will provide information in advance. The options are:

1. Make a selection button (command/argument/flag) for operators to choose whether they want their node to provide all or just some of the functions nodes have in the Nym Mixnet. Nodes functioning as Exit Gateways (in that epoch) will then have bigger rewards due to their larger risk exposure and overhead work with the setup.

2. All nodes will be required to have the Exit Gateway functionality. All nodes are rewarded the same as now, and the difference is that a node sometimes (some epochs) may be performing as Exit Gateway sometimes as Mix node or Entry Gateway adjusted according the network demand by an algorithm.

### Where can I read more about the Exit Gateway setup?

We created an [entire page](../../legal/exit-gateway.md) about the technical and legal questions around Exit Gateway.

### What is the change from allow list to deny list?

The operators running Gateways would have to “open” their nodes to a wider range of online services, in a similar fashion to Tor exit relays. The main change will be to expand the original short [`allowed.list`](https://nymtech.net/.wellknown/network-requester/standard-allowed-list.txt) to a more permissive setup. An [exit policy](https://nymtech.net/.wellknown/network-requester/exit-policy.txt) will constrain the hosts that the users of the Nym VPN and Mixnet can connect to. This will be done in an effort to protect the operators, as Gateways will act both as SOCKS5 Network Requesters, and exit nodes for IP traffic from Nym VPN and Mixnet clients.

### How will the Exit policy be implemented?

Follow the dynamic progress of exit policy implementation on Gateways below:

| **Step** | **Status** |
| :--- | :--- |
| **1.** By default the [exit policy](https://nymtech.net/.wellknown/network-requester/exit-policy.txt) filtering is disabled and the [`allowed.list`](https://nymtech.net/.wellknown/network-requester/standard-allowed-list.txt) filtering is going to continue be used. This is to prevent operators getting surprised by upgrading their Gateways (or Network Requesters) and suddenly be widely open to the internet. To enable the new exit policy, operators must use `--with-exit-policy` flag or modify the `config.toml` file. | ✅ done |
| **2.** The exit policy is part of the Gateway setup by default. To disable this exit policy, operators must use `--disable-exit-policy` flag. | ✅ done |
| **3.** The exit policy is the only option. The `allowed.list` is completely removed. | ✅ done |

Keep in mind the table above only relates to changes happening on Gateways. For the Project Smoosh progress refer to the [table above](./smoosh-faq.md#what-are-the-changes). Whether Exit Gateway functionality will be optional or mandatory part of every active Nym Node depends on the chosen [design](./smoosh-faq.md#what-does-it-mean-for-nym-nodes-operators).

### Can I run a Mix Node only?

It depends which [design](./smoosh-faq.md#what-does-it-mean-for-nym-nodes-operators) will ultimately be used. In case of the first - yes. In case of the second option, all the nodes will be setup with Exit Gateway functionality turned on.

## Token Economics & Rewards

```admonish info
For any specifics on Nym token economics and Nym Mixnet reward system, please read the [Nym token economics paper](https://nymtech.net/nym-cryptoecon-paper.pdf).
```

### What are the incentives for the node operator?

In the original setup there were no incentives to run a `nym-network-requester` binary. After the transition all the users will buy multiple tickets of zkNyms credentials and use those as [anonymous e-cash](https://arxiv.org/abs/2303.08221) to pay for their data traffic ([`Nym API`](https://github.com/nymtech/nym/tree/master/nym-api) will do the do cryptographical checks to prevent double-spending). All collected fees get distributed to all active nodes proportionally to their work by the end of each epoch.

### How does this change the token economics?

The token economics will stay the same as they are, same goes for the reward algorithm.

### How are the rewards distributed?

This depends on [design](./smoosh-faq.md#what-does-it-mean-for-nym-nodes-operators) chosen. In case of \#1, it will look like this:

As each operator can choose what roles their nodes provide, the nodes which work as open Gateways will have higher rewards because they are the most important to keep up and stable. Besides that the operators of Gateways may be exposed to more complication and possible legal risks.

The nodes which are initialized to run as Mix Nodes and Gateways will be chosen to be on top of the active set before the ones working only as a Mix Node.

I case we go with \#2, all nodes active in the epoch will be rewarded proportionally according their work.

In either way, Nym will share all the specifics beforehand.

### How will be the staking and inflation after project Smoosh?

Nym will run tests to count how much payment comes from the users of the Mixnet and if that covers the reward payments. If not, we may need to keep inflation on to secure incentives for high quality Gateways in the early stage of the transition.

### When project smooth will be launched, it would be the mixmining pool that will pay for the Gateway rewards based on amount of traffic routed ?

Yes, the same pool. Nym's aim is to do minimal modifications. The only real modification on the smart contract side will be to get into top X of 'active set' operators will need to have open Gateway function enabled.

### What does this mean for the current delegators?

From an operator standpoint, it shall just be a standard Nym upgrade, a new option to run the Gateway software on your node. Delegators should not have to re-delegate.

## Legal Questions

### Are there any legal concerns for the operators?

So far the general line is that running a Gateway is not illegal (unless you are in Iran, China, and a few other places) and due to encryption/mixing less risky than running a normal VPN node. For Mix Nodes, it's very safe as they have "no idea" what packets they are mixing.

There are several legal questions and analysis to be made for different jurisdictions. To be able to share resources and findings between the operators themselves we created a [Community Legal Forum](../../legal/exit-gateway.md).
