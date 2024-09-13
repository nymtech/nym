# Nym Nodes related Frequently Asked Questions

### What determines the rewards when running a `nym-node --mode mixnode`?

The stake required for a Mix Node to achieve maximum rewards is called Mix Node saturation point. This is calculated from the staking supply (all circulating supply + part of unlocked tokens). The target level of staking is to have 40% of the staking supply locked in Mix Nodes.

The node stake saturation point, which we denote by Nsat, is given by the stake supply, target level of staking divided between the rewarded nodes.

This design ensures the nodes aim to have a same size of stake (reputation) which can be done by delegation staking, as well as it secures a whale prevention and decentralization of staking, as any higher level of delegated $NYM than Nsat per node results in worsening reward ratio. On the contrary, the more Mix Nodes are active, the lower is Nsat. The equilibrium is reached when the staked tokens are delegated equally across the active Mix nodes and that's our basis for this incentive system.

<!--
<iframe src="https://status.notrustverify.ch/d-solo/CW3L7dVVk/nym-mixnet?orgId=1&from=1703074760986&to=1705666760986&panelId=5" width="800" height="400" frameborder="0"></iframe>
-->

The rewarded nodes are the nodes which will receive some rewards by the end of the given epoch. These can be separated further separated into:

1. Active: Top *N* nodes of the rewarded set (currently all of them but this can change), these are nodes which are used by the clients and mix packets.

2. Standby: Bottom *N* nodes of the rewarded set, they don't mix data from the clients but are used for testing. Their reward is smaller.


For more detailed calculation, read our blog post [Nym Token Economics update](https://blog.nymtech.net/nym-token-economics-update-fedff0ed5267). More info on staking can be found [here](https://blog.nymtech.net/staking-in-nym-introducing-mainnet-mixmining-f9bb1cbc7c36). And [here](https://blog.nymtech.net/want-to-stake-in-nym-here-is-how-to-choose-a-mix-node-to-delegate-nym-to-c3b862add165) is more info on how to choose a Mix Node for delegation. And finally an [update](https://blog.nymtech.net/quarterly-token-economic-parameter-update-b2862948710f) on token economics from July 2023.

<!--
<iframe src="https://status.notrustverify.ch/d-solo/CW3L7dVVk/nym-mixnet?orgId=1&from=1703074829887&to=1705666829887&panelId=31" width="850" height="400" frameborder="0"></iframe>
-->

<iframe src="https://dashboard.notrustverify.ch/d-solo/l71MWkX7k/ntv-mixnode?orgId=1&from=1710949572440&to=1713537972440&panelId=18" width="850" height="400" frameborder="0"></iframe>

*More graphs and stats at [stats.notrustverify.ch](https://status.notrustverify.ch/d/CW3L7dVVk/nym-mixnet?orgId=1&from=1703074861988&to=1705666862004).*


