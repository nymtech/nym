# Nym Operators Rewards

```admonish info title="\* Important Info"
**The data on this page were last time updated on <!--cmdrun cd ../../../scripts/cmdrun && ./api_targets.py time_now-->. Every information labeled with `*` sign is corresponding to that time stamp.**
```

* Nym tokenomics are based on the research paper [*Reward Sharing for Mixnets*](https://nymtech.net/nym-cryptoecon-paper.pdf)
* For a more comprehensive overview, live data and supply graphs, visit [*nymtech.net/about/token*](https://nymtech.net/about/token)

**Formulas and Examples Annotation**

To make it easier for the reader, we use a highlighting line on the left side, with a specific color:

```admonish tip title=""
$Green\ for\ formulas.$
```

```admonish example collapsible=true
$Purple\ collapsible\ for\ examples.$
```

## Overview

This is a quick summarry, to understand the full picture, please see detailed [*Rewards Logic & Calculation*](#rewards-logic--calculation) chapter below.

* The operators of `nym-node` get rewarded from Mixmining pool, which emits around 6000 NYM per hour.
* An active set of `nym-nodes` selected for Nym network routing and mixing is 240 nodes in total and it's selected for each new epoch (60 min).
* The active set is composed of 120 Mixnodes and 120 Gateways.
* `nym-nodes` can run in mode `entry-gateway`, `exit-gateway` and `mixnode`, the active set selection of each mode is

| Nym node mode   | Total in active set | Rationale                                                 |
| :---            | ---:                | :---                                                      |
| `mixnode`       | 120                 | Always 3 layers of 40 mixnodes                            |
| `entry-gateway` | 120                 | Any Gateway can act as an entry                           |
| `exit-gateway`  | max 120             | Any Gateway running `exit-gateway` mode from the same 120 |

* NymVPN can route through Nym Network two ways, both using the same active set:
	- Mixnet: 5 layers routing and mixing - full privacy
	- Wireguard: 2 layers routing, skipping 3 mixing layers - fast mode
* The reward distribution is per layer according to a [decision made by the operators](https://forum.nymtech.net/t/poll-what-should-be-the-split-of-mixmining-rewards-among-the-layers-of-the-nym-mixnet/407) as follows:
	- 5-hop: 16%-16%-16%-16%-36%
	- 2-hop: (In future) 33%-67%
	- Currently Gateways earn rewards only from 5-hop mode. The operators can sign up to a grant program as a substitution for 2-hop routing.
* Each node is rewarded according to the layer in which it's positioned in the given epoch, divided uniformly between all nodes in that layer.
* Nodes are selected to the active set based on their performance and stake saturation (slef bond + delegation)
* In future a ticket system will be implemented where nodes will be rewarded according to the work they perform, with a revenue for both 2-hop and 5-hop work. The uniform naive distribution is an intermediate step. See [*Roadmap*](#roadmap) chapter for more details.

## Rewards Logic & Calculation

**Note that in the current intermediate model we use one active set for both 2-hop and 5-hop routing Gateways. In reality it means that all nodes are rewarded only within 5-hop reward scheme only. In the meantime, given that the 120 Gateways within the active set route traffic for 2-hop wireguard mode as well, without any extra rewards, the operators can get substitution in the form of grants.**

~~~admonish tip title="Nym network active set distribution"
```ascii

 Network
 layer:           1.           2.           3.           4.           5.


                            ┌► mixnode ─┐   mixnode      mixnode
                            │           │
 Node             entry     │           │                             exit
 type:            gateway ──┘  mixnode  │   mixnode  ┌─► mixnode ───► gateway
                                        │            │
                                        │            │
                               mixnode  └─► mixnode ─┘   mixnode



 Active set:      120           40           40           40      max 120


 Rewards
 distribution     16%          16%          16%          16%          36%
 5-hop mode:


 Future implementation:

 Rewards
 distribution     33%          skip         skip         skip         67%
 2-hop mode:

```
~~~

### Active Set Selection

*Performance matters!*

For a node to be rewarded, the node must be part of an active set in the first place. The active set is selected in the beginning of each epoch (every 60min) where total of 240 nodes - represented by 120 mixnodes and 120 gateways, are randomly allocated across the layers. Mixnodes only work within the given layer, while any Exit Gateway can be chosen by a client as an Entry Gateway, not vice versa.

The algorithm choosing nodes into the active set takes into account node's performance and stake saturation, both values being between 0 and 1.

```admonish tip title=""
$$
active\ set\ selection\ probability = node\ performance^{20} * stake\ saturation
$$
```

For a comparison we made an example with 5 nodes, where first number is node performance and second stake saturation:

```admonish example collapsible=true
$$
\begin{align}
\notag node_1 &= 1^{20} * 1 = 1 \\
\notag node_2 &= 1^{20} * 0.5 = 0.5 \\
\notag node_3 &= 0.99^{20} * 1 = 0.818 \\
\notag node_4 &= 0.95^{20} * 1 = 0.358 \\
\notag node_5 &= 0.9^{20} * 1 = 0.122 \\
\end{align}
$$
```

As you can see the performance (also known as *Routing score*) is much more important during the active set selection. A node with 100% performance but only 50% stake saturation has much bigger chance to be chosen than a node with 95% performance but full stake saturation and incomparably bigger chance than 90% performing node with 100% stake saturation.


### Layer Distribution

Once the active set of 120 Mixnodes and 120 Gateways is selected, the nodes can start to route and mix packets in the Nym Network. Each hour a total of 6000 NYM is distributed between the layers from Mixmining pool, following the ratio according to a [decision made by the operators](https://forum.nymtech.net/t/poll-what-should-be-the-split-of-mixmining-rewards-among-the-layers-of-the-nym-mixnet/407) as follows:

```admonish tip title=""
5-hop mixnet mode: <br>
$16\%; 16\%; 16\%; 16\%; 36\%$ <br>
<!-- COMMENTING OUT FOR NOW AS WE DON'T HAVE IT IMPLEMENTED
2-hop wireguard mode: $33\%; 67\%$
-->
```

In real numbers: If hourly revenue to all 240 nodes is 6000 NYM, the layer compartmentalisation is 960 NYM for Entry Gateway layer and each Mixnode layer and 2160 NYM for Exit Gateway layer. The calculation is in the example below:

```admonish example collapsible=true
5-hop mixnet mode: <br>
$0.16 * 6000; 0.16 * 6000; 0.16 * 6000; 0.16 * 6000; 0.36 * 6000 = 960; 960; 960; 960; 2160$ <br>
<!-- COMMENTING OUT AS WE DO NOT HAVE A CLEAR NUMBERS HERE
<br>
2-hop wireguard mode:<br>
$33\% - 67\%$
-->
```

### Node Rewards within Same Layer


### Operation Cost, Profit Margin & Delegation


## Roadmap

<!-- PUT FINAL TOKENOMIC SCHEME AND ALL STEPS TOWARDS IT IN HERE -->

## Stats

NYM token is capped at 1b. Below is a table with actual\* token supply distribution.

<!--cmdrun cd ../../../scripts/cdmrun && ./api_targets.py s --api mainnet --endpoint circulating-supply --format -->


<!-- ADD MIXNET STATS GRAPHS -->


<!-- DROPPING THIS FROM THE MAINTENANCE PAGE - NEEDS REWORK -->

## Mix Node Reward Estimation API endpoint

<!-- THIS NEEDS REDO -->

The Reward Estimation API endpoint allows Mix Node operators to estimate the rewards they could earn for running a Nym Mix Node with a specific `MIX_ID`.

> The `<MIX_ID>` can be found in the "Mix ID" column of the [Harbourmaster](https://harbourmaster/nymtech.net).

<!--
The endpoint is a particularly common for Mix Node operators as it can provide an estimate of potential earnings based on factors such as the amount of traffic routed through the Mix Node, the quality of the Mix Node's performance, and the overall demand for Mix Nodes in the network. This information can be useful for Mix Node operators in deciding whether or not to run a Mix Node and in optimizing its operations for maximum profitability.
-->

We have available API endpoints which can be accessed via [Swagger UI page](https://validator.nymtech.net/api/swagger/index.html). Or by querying the endpoints directly:

```sh
curl -X 'GET' \
  'https://validator.nymtech.net/api/v1/status/mixnode/<MIX_ID>/reward-estimation' \
  -H 'accept: application/json'sh
```

Query response will look like this:

```sh
    "estimation": {
        "total_node_reward": "942035.916721770541325331",
        "operator": "161666.263307386408152071",
        "delegates": "780369.65341438413317326",
        "operating_cost": "54444.444444444444444443"
    },
```

> The unit of value is measured in `uNYM`.

```admonish tip title=""
$1 \ NYM = 1 \_ 000 \_ 000 \ uNYM$
```
- `estimated_total_node_reward` - An estimate of the total amount of rewards that a particular Mix Node can expect to receive during the current epoch. This value is calculated by the Nym Validator based on a number of factors, including the current state of the network, the number of Mix Nodes currently active in the network, and the amount of network traffic being processed by the Mix Node.

- `estimated_operator_reward` - An estimate of the amount of rewards that a particular Mix Node operator can expect to receive. This value is calculated by the Nym Validator based on a number of factors, including the amount of traffic being processed by the Mix Node, the quality of service provided by the Mix Node, and the operator's stake in the network.

- `estimated_delegators_reward` - An estimate of the amount of rewards that Mix Node delegators can expect to receive individually. This value is calculated by the Nym Validator based on a number of factors, including the amount of traffic being processed by the Mix Node, the quality of service provided by the Mix Node, and the delegator's stake in the network.

- `estimated_node_profit` - An estimate of the profit that a particular Mix node operator can expect to earn. This value is calculated by subtracting the Mix Node operator's `operating_costs` from their `estimated_operator_reward` for the current epoch.

- `estimated_operator_cost` - An estimate of the total cost that a particular Mix Node operator can expect to incur for their participation. This value is calculated by the Nym Validator based on a number of factors, including the cost of running a Mix Node, such as server hosting fees, and other expenses associated with operating the Mix Node.
