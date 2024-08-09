# Fair Mixnet

```admonish info title="\*Info"
**The data on this page were last time updated on <!--cmdrun cd ../../../scripts/cmdrun && ./api_targets.py time_now-->.**
```

Nym Network is composed of two main elements, the Mixnet represented by [Nym Nodes](../nodes/nym-node.md) routing and mixing the data packets, and Nyx blockchain based on [validator set](validator-rewards.md), using smart contracts (based on [cosmwasm]()) to monitor and reward Nym Nodes by querying API endpoints and distributing NYM token to operators from Mixmining pool. All Nym nodes and validators are run by decentralised community of operators.

* Nym tokenomics are based on the research paper [*Reward Sharing for Mixnets*](https://nymtech.net/nym-cryptoecon-paper.pdf)
* For a more comprehensive overview, live data and supply graphs, cisit [nymtech.net/about/token](https://nymtech.net/about/token)

**Formulas and Examples Annotation**

To make it easier for the reader, we will use a highlighting line on the left side, with a specific color:

```admonish tip title=""
Green for formulas.
```

```admonish example collapsible=true
Purple collapsible for examples.
```

## NYM Tokenomics

Besides the Mixnet itself, Nym Network is based on its own blockchain Nyx (IBC on Cosmos) with a native token NYM. NYM token key features are:

* **Incentives:** Distribute rewards to decentralised nodes based on mixing and routing (work). This dynamic ensures that the network is as robust as possible - the nodes are chosen every hour according to their performance.

* **Network take over defense:** Another decisive factore for a node to be chosen to the network active set is reputation. Reputation is a size of stake (delegation) where delegators earn proportional percentage of nodes rewards.

* **Centralisation defense:** Any node can only have a certain stake (called stake saturation) to earn maximum rewards, increasing stake level per node leads to decreasing rewards for the operator and all delegators. This feature makes it more difficult for whales to over-stake their nodes or to attract more delegators (stakers) as they would become dis-advantaged.

To learn more about rewards calculation and distributtion, read the next page [*Nym Operators Rewards*](mixnet-rewards.md).


### Utility

*NYM token is a first and foremost a utility to secure Nym Network.*

Nyx blockchain's validators run API to monitor the network and node performance. Based on the live input the operators and stakers of the active nodes get rewarded and the network is adjusted and re-randomized in the beginning of each epoch (60 min) using cosmwasm smart contracts.

On one hand node operators get [rewarded](mixnet-rewards.md) for the work they do, but their revenue is not forgranted. Only best performing nodes with a solid reputation can take part in the network. This creates an incentive for people to operate Nym nodes as quality and reliable service. The reputation system also works as a network defense against a large adversary take over or sybil attacks.

On the other, node reputation is calculated by delegation. Delegation is a stake done by NYM token holders on top of nodes they want to support to join the network as it compensate the stakers with APR. Therefore there is an incetive for NYM holders to stake their token on top of nodes which they believe will perform well. To prevent a whale take-over and centralisation, the revenue grows alongside nodes stake size only until a certain point, after which the rewards per staker start to decrease. We call this mark *node stake saturation*.

Thanks to Nyx blockchain API monitoring, the flow is dynamic and constantly optimized based on live metrics. Below is a detailed explanation and reckoning of Nym tokenomics logic.

### Tokenomics

Before we can arrive to a full comprehension of [node operators rewards](mixnet-rewards.md) and [delegators APR height](https://nymtech.net/about/token) we need to understand some basic logic and stats of Nym token economics. All the data can be [queryied from valdator API](#query-tokenomics-api).

* **Supply:** NYM token is capped at 1b. Visit [Nym token page](https://nymtech.net/about/token) to see live data and graphs. Current\* circulating supply is <-- cmdrun cd ../../../scripts/cmdrun && ./api_targets.py v --api mainnet --endpoint circulating-supply --value circulating_supply amount ---> NYM.

* **Staking target:** A number of aimed NYM tokens to be staked in the network. This number can be changed to optimize the following metrics below, currently\* it's set to be <!--cmdrun cd ../../../scripts/cmdrun && ./api_targets.py v --api mainnet --endpoint epoch/reward_params --value interval staking_supply_scale_factor --format percent -->.

* **Stake saturation:** Node reputation in a form of self bond or stakers delegation. Stake saturation is calculated as:
```admonish tip title=""
$stake\ saturation = staking\ target\ /\ total\ \#\ of\ nodes$
```
<!-- CODE AUTO COMPLETION:
- # of nodes in the network
- circulating supply * staking target
- staking target / # of nodes in the network
-->

With current\* circulating supply of <!-- cmdrun cd ../../../scripts/cmdrun && ./api_targets.py v --api mainnet --endpoint circulating-supply --value circulating_supply amount --> NYM, stake saturation of <!-- cmdrun cd ../../../scripts/cmdrun &&    --> and <!-- cmdrun cd ../../../scripts/cmdrun &&     --> nodes bonded in Nym Network, the stake saturation level is <!-- cmdrun cd ../../../scripts/cmdrun &&     --> NYM per node.


### Summary in Numbers

Below is a table with Nyx chain data\* and token supply distribution.

<!--cmdrun cd ../../../scripts/cmdrun && ./api_targets.py v --api mainnet --endpoint circulating-supply --format markdown --separator _ -->

To get live data, visit [Nym token page](https://nymtech.net/about/token
) or see how to [query API endpoints](#query-tokenomics-api).

## Query Tokenomics API

<!-- MAKE A QUICK GUIDE TO QUESRY THE STATS -->

https://validator.nymtech.net/api/v1/circulating-supply

https://validator.nymtech.net/api/v1/epoch/reward_params
