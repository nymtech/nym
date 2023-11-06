# Frequently Asked Questions

## Mixnet nodes

### What determines the rewards when running a mix node?

The stake required for a mix node to achieve maximum rewards is called mix node saturation point. This is calculated from the staking supply (all circulating supply + part of unlocked tokens). The target level of staking is to have 50% of the staking supply locked in mix nodes.

The node stake saturation point, which we denote by Nsat, is given by the stake supply, target level of staking divided by the number of rewarded (active) nodes. 

This design ensures the nodes aim to have a same size of stake (reputation) which can be done by delegation staking, as well as it ensures that there is a decentralization of staking as any higher level of staked tokens per node results in worse rewards. On the contrary, the more mix nodes are active, the lower is Nsat. The equilibrium is reached when the staked tokens are delegated equally across the active mix-nodes and that's our basis for this incentive system.

For more detailed calculation, read our blog post [Nym Token Economics update](https://blog.nymtech.net/nym-token-economics-update-fedff0ed5267). More info on staking can be found [here](https://blog.nymtech.net/staking-in-nym-introducing-mainnet-mixmining-f9bb1cbc7c36). And [here](https://blog.nymtech.net/want-to-stake-in-nym-here-is-how-to-choose-a-mix-node-to-delegate-nym-to-c3b862add165) is more info on how to choose a mix node for delegation. And finally an [update](https://blog.nymtech.net/quarterly-token-economic-parameter-update-b2862948710f) on token economics from July 2023.

### Which VPS providers would you recommend?

Consider in which jurisdiction you reside and where do you want to run a mix node. Do you want to pay by crypto or not and what are the other important particularities for your case? We always recommend operators to try to choose smaller and decentralised VPS providers over the most known ones controlling a majority of the internet. We receive some good feedback on these: Linode, Ghandi, Flokinet and Exoscale. Do your own research and share with the community.

<!---### Why is a mix node setup on a self-hosted machine so tricky?--->

### What's the Sphinx packet size?

The sizes are shown in the configs [here](https://github.com/nymtech/nym/blob/1ba6444e722e7757f1175a296bed6e31e25b8db8/common/nymsphinx/params/src/packet_sizes.rs#L12) (default is the one clients use, the others are for research purposes, not to be used in production as this would fragment the anonymity set). More info can be found [here](https://github.com/nymtech/nym/blob/4844ac953a12b29fa27688609ec193f1d560c996/common/nymsphinx/anonymous-replies/src/reply_surb.rs#L80).

### Why a mix node and a gateway cannot be bond to the same wallet?

Because of the way the smart contract works we keep it one-node one-address at the moment.

### Which nodes are the most needed to be setup to strengthen Nym infrastructure and which ones bring rewards?

Right now only mix nodes are rewarded. We're working on gateway and service payments. Gateways are the weak link right now due mostly to lack of incentivisation. Services like Network Requesters are obviously the most necessary for people to start using the platform, and we're working on smart contracts to allow for people to start advertising them the same way they do mix nodes.

### Are mixnodes whitelisted?

Nope, anyone can run a mix node. Purely reliant on the node's reputation (self stake + delegations) & routing score.

## Validators and tokens

### What's the difference between NYM and uNYM?

1 NYM = 1 000 000 uNYM

<!--- Commenting for now as NYX is not publicly out yet
### What's the difference between NYM and NYX?
--->

### Can I run a validator?

We are currently working towards building up a closed set of reputable validators. You can ask us for coins to get in, but please don't be offended if we say no - validators are part of our system's core security and we are starting out with people we already know or who have a solid reputation.

### Why is validator set entry whitelisted?

We understand that the early days of the Nyx blockchain will face possible vulnerabilities in terms of size - easy to disrupt or halt the chain if a malicious party entered with a large portion of stake. Besides that, there are some legal issues we need to address before we can distribute the validator set in a fully permissions fashion.

### Why does Nym do many airdrops?

It is part of ensuring decentralisation - we need to avoid a handful of people having too much control over the token and market. Of course ideally people will stake the tokens and contribute to the project at this stage. We run surveys to better understand what people are doing with their tokens and what usability issues there are for staking. Any feedback is appreciated as it helps us improve all aspects of using the token and participating in the ecosystem.

