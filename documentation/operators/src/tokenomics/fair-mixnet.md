# Fair Mixnet

```admonish info title="\*Info"
**The data on this page were last time updated on <!--cmdrun cd ../../../scripts/cmdrun && ./api_targets.py time_now-->.**
```

Nym Network is composed of two main elements, the Mixnet represented by [Nym Nodes](../nodes/nym-node.md) routing and mixing the data packets, and Nyx blockchain based on [validator set](validator-rewards.md), using smart contracts (based on [cosmwasm]()) to monitor and reward Nym Nodes by querying API endpoints and distributing NYM token to operators from Mixmining pool. All Nym nodes and validators are run by decentralised community of operators.

* Nym tokenomics are based on the research paper [*Reward Sharing for Mixnets*](https://nymtech.net/nym-cryptoecon-paper.pdf)
* For a more comprehensive overview, live data and supply graphs, cisit [nymtech.net/about/token](https://nymtech.net/about/token)

## NYM Token

Besides the Mixnet itself, Nym Network is based on its own blockchain Nyx (IBC on Cosmos) with a native token NYM. NYM token key features are:

* **Incentives:** Distribute rewards to decentralised nodes based on mixing and routing (work). This dynamic ensures that the network is as robust as possible - the nodes are chosen every hour according to their performance.

* **Network take over defense:** Another decisive factore for a node to be chosen to the network active set is reputation. Reputation is a size of stake (delegation) where delegators earn proportional percentage of nodes rewards.

* **Centralisation defense:** Any node can only have a certain stake (called stake saturation) to earn maximum rewards, increasing stake level per node leads to decreasing rewards for the operator and all delegators. This feature makes it more difficult for whales to over-stake their nodes or to attract more delegators (stakers) as they would become dis-advantaged.

To learn more about rewards calculation and distributtion, read the next page [*Nym Operators Rewards*](mixnet-rewards.md).

### Supply & Distribution

NYM token is capped at 1b. Below is a table with actual\* token supply distribution.

<!--cmdrun cd ../../../scripts/cmdrun && ./api_targets.py s --api mainnet --endpoint circulating-supply --format --separator _ -->

To get live data, visit [Nym token page](https://nymtech.net/about/token
) or see `/circulating-supply` [API endpoint](https://validator.nymtech.net/api/v1/circulating-supply).
