# Frequently Asked Questions

## Nym Mixnet

To see all the stats about Nym Mixnet live, we recommend you to visit [status.notrustverify.ch](https://status.notrustverify.ch/d/CW3L7dVVk/nym-mixnet?orgId=1) built by [No Trust Verify](https://notrustverify.ch/) crew, one of the squads within Nym core community.

<iframe src="https://status.notrustverify.ch/d-solo/CW3L7dVVk/nym-mixnet?orgId=1&from=1702215592419&to=1704807592419&panelId=12" width="800" height="400" frameborder="0"></iframe>


### Is there an explorer for Nym Mixnet?

Yes, there are several places, some are built by Nym core community:

* [Nym Explorer](https://explorer.nymtech.net/)
* [Guru Explorer](https://mixnet.explorers.guru/)
* [ExploreNYM](https://explorenym.net/)

### What determines the rewards when running a Mix Node?

The stake required for a Mix Node to achieve maximum rewards is called Mix Node saturation point. This is calculated from the staking supply (all circulating supply + part of unlocked tokens). The target level of staking is to have 50% of the staking supply locked in Mix Nodes.

The node stake saturation point, which we denote by Nsat, is given by the stake supply, target level of staking divided by the number of rewarded (active) nodes.

This design ensures the nodes aim to have a same size of stake (reputation) which can be done by delegation staking, as well as it ensures that there is a decentralization of staking as any higher level of staked tokens per node results in worse rewards. On the contrary, the more Mix Nodes are active, the lower is Nsat. The equilibrium is reached when the staked tokens are delegated equally across the active Mix nodes and that's our basis for this incentive system.

For more detailed calculation, read our blog post [Nym Token Economics update](https://blog.nymtech.net/nym-token-economics-update-fedff0ed5267). More info on staking can be found [here](https://blog.nymtech.net/staking-in-nym-introducing-mainnet-mixmining-f9bb1cbc7c36). And [here](https://blog.nymtech.net/want-to-stake-in-nym-here-is-how-to-choose-a-mix-node-to-delegate-nym-to-c3b862add165) is more info on how to choose a Mix Node for delegation. And finally an [update](https://blog.nymtech.net/quarterly-token-economic-parameter-update-b2862948710f) on token economics from July 2023.

### Which VPS providers would you recommend?

Consider in which jurisdiction you reside and where do you want to run a Mix Node. Do you want to pay by crypto or not and what are the other important particularities for your case? We always recommend operators to try to choose smaller and decentralised VPS providers over the most known ones controlling a majority of the internet. We receive some good feedback on these: Linode, Ghandi, Flokinet and Exoscale. Do your own research and share with the community.

### Why is a mix node setup on a self-hosted machine so tricky?

We don't recommend this setup because it's really difficult to get a stable IPv6 from most of the ISPs.

### What's the Sphinx packet size?

The sizes are shown in the configs [here](https://github.com/nymtech/nym/blob/1ba6444e722e7757f1175a296bed6e31e25b8db8/common/nymsphinx/params/src/packet_sizes.rs#L12) (default is the one clients use, the others are for research purposes, not to be used in production as this would fragment the anonymity set). More info can be found [here](https://github.com/nymtech/nym/blob/4844ac953a12b29fa27688609ec193f1d560c996/common/nymsphinx/anonymous-replies/src/reply_surb.rs#L80).

### Why a Mix Node and a Gateway cannot be bonded with the same wallet?

Because of the way the smart contract works we keep it one-node one-address at the moment.

### Which nodes are the most needed to be setup to strengthen Nym infrastructure and which ones bring rewards?

Ath this point the most crutial component needed are [Exit Gateways](../legal/exit-gateway.md).

### Are Mix Nodes whitelisted?

Nope, anyone can run a Mix Node. Purely reliant on the node's reputation (self stake + delegations) & routing score.

## Validators and tokens

### What's the difference between NYM and uNYM?

1 NYM = 1 000 000 uNYM

<!--- Commenting for now as NYX is not publicly out yet
### What's the difference between NYM and NYX?
--->

### Why some Nyx blockchain operations take one hour and others are instant?

This is based on the definition in [Nym's CosmWasm](https://github.com/nymtech/nym/tree/develop/common/cosmwasm-smart-contracts) smart contracts code.

Whatever is defined as [a pending epoch event](https://github.com/nymtech/nym/blob/b07627d57e075b6de35b4b1a84927578c3172811/common/cosmwasm-smart-contracts/mixnet-contract/src/pending_events.rs#L35-L103) will get resolved at the end of the current epoch.

And whatever is defined as [a pending interval event](https://github.com/nymtech/nym/blob/b07627d57e075b6de35b4b1a84927578c3172811/common/cosmwasm-smart-contracts/mixnet-contract/src/pending_events.rs#L145-L172) will get resolved at the end of the current interval.

### Can I run a validator?

We are currently working towards building up a closed set of reputable validators. You can ask us for coins to get in, but please don't be offended if we say no - validators are part of our system's core security and we are starting out with people we already know or who have a solid reputation.

### Why is validator set entry whitelisted?

We understand that the early days of the Nyx blockchain will face possible vulnerabilities in terms of size - easy to disrupt or halt the chain if a malicious party entered with a large portion of stake. Besides that, there are some legal issues we need to address before we can distribute the validator set in a fully permissions fashion.

### Why does Nym do many airdrops?

It is part of ensuring decentralisation - we need to avoid a handful of people having too much control over the token and market. Of course ideally people will stake the tokens and contribute to the project at this stage. We run surveys to better understand what people are doing with their tokens and what usability issues there are for staking. Any feedback is appreciated as it helps us improve all aspects of using the token and participating in the ecosystem.
