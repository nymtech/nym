# Nym Operators Rewards

```admonish info
**The data on this page were last time updated on <!--cmdrun cd ../../../scripts/cmdrun && ./api_targets.py time_now-->.**
```
<!--
ADD:

PAGE: https://nymtech.net/about/token
WP: https://nymtech.net/nym-cryptoecon-paper.pdf


Rewards split:
16-16-16-16-36
33-67
https://forum.nymtech.net/t/poll-what-should-be-the-split-of-mixmining-rewards-among-the-layers-of-the-nym-mixnet/407
-->



## Supply

<!--cmdrun cd ../../../scripts/cdmrun && ./api_targets.py s --api mainnet --endpoint circulating-supply --format -->








<!-- DROPPING THIS FROM THE MAINTENANCE -->

### Mix Node Reward Estimation API endpoint

The Reward Estimation API endpoint allows Mix Node operators to estimate the rewards they could earn for running a Nym Mix Node with a specific `MIX_ID`.

> The `<MIX_ID>` can be found in the "Mix ID" column of the [Network Explorer](https://explorer.nymtech.net/network-components/mixnodes/active).

The endpoint is a particularly common for Mix Node operators as it can provide an estimate of potential earnings based on factors such as the amount of traffic routed through the Mix Node, the quality of the Mix Node's performance, and the overall demand for Mix Nodes in the network. This information can be useful for Mix Node operators in deciding whether or not to run a Mix Node and in optimizing its operations for maximum profitability.

Using this API endpoint returns information about the Reward Estimation:

```sh
/status/mixnode/<MIX_ID>/reward-estimation
```

Query Response:

```sh
    "estimation": {
        "total_node_reward": "942035.916721770541325331",
        "operator": "161666.263307386408152071",
        "delegates": "780369.65341438413317326",
        "operating_cost": "54444.444444444444444443"
    },
```

> The unit of value is measured in `uNYM`.

- `estimated_total_node_reward` - An estimate of the total amount of rewards that a particular Mix Node can expect to receive during the current epoch. This value is calculated by the Nym Validator based on a number of factors, including the current state of the network, the number of Mix Nodes currently active in the network, and the amount of network traffic being processed by the Mix Node.

- `estimated_operator_reward` - An estimate of the amount of rewards that a particular Mix Node operator can expect to receive. This value is calculated by the Nym Validator based on a number of factors, including the amount of traffic being processed by the Mix Node, the quality of service provided by the Mix Node, and the operator's stake in the network.

- `estimated_delegators_reward` - An estimate of the amount of rewards that Mix Node delegators can expect to receive individually. This value is calculated by the Nym Validator based on a number of factors, including the amount of traffic being processed by the Mix Node, the quality of service provided by the Mix Node, and the delegator's stake in the network.

- `estimated_node_profit` - An estimate of the profit that a particular Mix node operator can expect to earn. This value is calculated by subtracting the Mix Node operator's `operating_costs` from their `estimated_operator_reward` for the current epoch.

- `estimated_operator_cost` - An estimate of the total cost that a particular Mix Node operator can expect to incur for their participation. This value is calculated by the Nym Validator based on a number of factors, including the cost of running a Mix Node, such as server hosting fees, and other expenses associated with operating the Mix Node.

### Validator: Installing and configuring nginx for HTTPS
#### Setup
[Nginx](https://www.nginx.com/resources/glossary/nginx) is an open source software used for operating high-performance web servers. It allows us to set up reverse proxying on our validator server to improve performance and security.

Install `nginx` and allow the 'Nginx Full' rule in your firewall:
