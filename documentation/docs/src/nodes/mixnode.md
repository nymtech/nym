# Mix Nodes

> The mix node setup and maintenance guide has migrated to the [Operator Guides book](https://nymtech.net/developers/nodes/mix-node-setup.html).

Mix nodes are the backbone of the mixnet. These are the nodes that perform 'mix mining', otherwise known simply as 'mixing'. 

Mix nodes, after receiving a packet, decrypt its outer 'layer', and hold them for a variable amount of time before passing them to their next destination - either another mix node, or a gateway. In doing so, they 'mix' packets by sending them to their next destination in a different order than they were received. 

Mix nodes are rewarded according to their quality of service, and their inclusion in the active set (i.e. the nodes that mix traffic for the next epoch) affected by this (as well as their delegation-based reputation - see [further reading](#further-reading) for more on this). 

## (Coming soon) Mixing: a Step-by-Step Breakdown

## Further reading
* [Nym Whitepaper](https://nymtech.net/nym-whitepaper.pdf) section 4 
* [Nym Blog: Mix node deepdive](https://blog.nymtech.net/nym-mixnodes-deep-dive-d2b91917f097)
* [Mixnet Traffic Flow overview](../architecture/traffic-flow.md)
* [Reward Sharing for Mixnets](https://nymtech.net/nym-cryptoecon-paper.pdf)