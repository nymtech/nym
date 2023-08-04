# Mix Nodes

> The mix node setup and maintenance guide has migrated to the [Operator Guides book](https://nymtech.net/developers/nodes/mix-node-setup.html).

Mix nodes are the backbone of the mixnet. These are the nodes that perform 'mix mining', otherwise known simply as 'mixing' or performing the 'poisson mix'. 

Mix nodes perform one task: receiving packets, decrypting their outer 'layer', and holding them for a variable amount of time before passing them to their next destination - either another mix node, or a gateway. In doing so, they 'mix' packets by sending them to their next destination in a different order than they were recieved.

The aim of this mixing is to protect against timing-based deanonymisation attempts by a global adversary able to monitor the entire network with a 'God's Eye View'. 

Mix nodes are rewarded according to their quality of service, and their inclusion in the active set (i.e. the nodes that mix traffic for the next epoch) affected by this (as well as their delegation-based reputation - see [further reading](#further-reading) for more on this). 

## (Coming soon) Mixing: a Step-by-Step Breakdown

## Further reading
TODO 
* TODO SECTION OF WHITEPAPER
* TODO LOOPIX PAPER SECTION 
* <LINK TO CODEBASE> 
* <ANY VIDEOS>
