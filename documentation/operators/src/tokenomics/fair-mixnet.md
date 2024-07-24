# Fair Mixnet

```admonish info
**The data on this page were last time updated on <!--cmdrun cd ../../../scripts/cmdrun && ./api_targets.py time_now-->.**
```

Nym Network is composed of two main elements, the Mixnet represented by [Nym Nodes](../nodes/nym-node.md) routing and mixing the data packets, and Nyx blockchain based on [validator set](validator-rewards.md), using smart contracts (based on [cosmwasm]()) to monitor and reward Nym Nodes by querying API endpoints and distributing NYM token to operators from Mixmining pool.

MENTION WHITEPAPER HERE

## NYM Token

Besides the Mixnet itself, Nym Network is based on it's own blockchain Nyx (IBC on Cosmos) with a native token NYM. NYM token key features are:

* **Incentives:** Distribute rewards to decentralised nodes based on mixing and routing (work). This dynamic ensures that the network is as robust as possible - the nodes are chosen every hour according to their performance.

* **Network take over defense:** Nodes are chosen to the active set according to their reputation which is done through delegation (stake) where delegators earn proportional percentage of nodes rewards.

* **Centralisation defense:** Any node can only have a certain stake (called stake saturation) to earn maximum rewards, increasing stake level per node leads to decreasing rewards for the operator and all delegators. This feature makes it more difficult for whales to attract more delegators (stakers) as they would become dis-advantaged.

To learn more about rewards calculation and distributtion, read the next page [*Nym Operators Rewards*](mixnet-rewards.md).

### Supply

NYM token is

<!--cmdrun cd ../../../scripts/cmdrun && ./api_targets.py s --api mainnet --endpoint circulating-supply --format -->
