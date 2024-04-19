# General Operators FAQ

## Nym Mixnet

To see different stats about Nym Mixnet live, we recommend you to visit [status.notrustverify.ch](https://status.notrustverify.ch/d/CW3L7dVVk/nym-mixnet?orgId=1) built by [No Trust Verify](https://notrustverify.ch/) crew, one of the squads within Nym core community.

<iframe src="https://status.notrustverify.ch/d-solo/CW3L7dVVk/nym-mixnet?orgId=1&from=1702215592419&to=1704807592419&panelId=12" width="800" height="400" frameborder="0"></iframe>


### Is there an explorer for Nym Mixnet?

Yes, there are several places, some are built by Nym core community:

* [Nym Explorer](https://explorer.nymtech.net/)
* [Guru Explorer](https://mixnet.explorers.guru/)
* [ExploreNYM](https://explorenym.net/)

### Which VPS providers would you recommend?

Consider in which jurisdiction you reside and where do you want to run a Mix Node. Do you want to pay by crypto or not and what are the other important particularities for your case? We always recommend operators to try to choose smaller and decentralised VPS providers over the most known ones controlling a majority of the internet. We receive some good feedback on these: Linode, Ghandi, Flokinet and Exoscale. Do your own research and share with the community.

### Why is a node setup on a self-hosted machine so tricky?

We don't recommend this setup because it's really difficult to get a static IP and route IPv6 traffic.

### What's the Sphinx packet size?

The sizes are shown in the configs [here](https://github.com/nymtech/nym/blob/1ba6444e722e7757f1175a296bed6e31e25b8db8/common/nymsphinx/params/src/packet_sizes.rs#L12) (default is the one clients use, the others are for research purposes, not to be used in production as this would fragment the anonymity set). More info can be found [here](https://github.com/nymtech/nym/blob/4844ac953a12b29fa27688609ec193f1d560c996/common/nymsphinx/anonymous-replies/src/reply_surb.rs#L80).

### Why a Mix Node and a Gateway cannot be bonded with the same wallet?

Because of the way the smart contract works we keep it one-node one-address at the moment.

### Which nodes are the most needed to be setup to strengthen Nym infrastructure and which ones bring rewards?

Ath this point the most crutial component needed are [Exit Gateways](../legal/exit-gateway.md).

### Are Nym Nodes whitelisted?

Nope, anyone can run a nyx Node. Purely reliant on the node's reputation (self stake + delegations) & routing score.


