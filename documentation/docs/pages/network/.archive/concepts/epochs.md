# Epochs

Time in the context of the Mixnet is organised into epochs. The length of an epoch is configurable but currently set to be one hour.

Several actions happen per epoch:
- Reward calculation and distribution for Nym Nodes. See the [Operator Docs](../../operators/tokenomics/mixnet-rewards) for more information on reward calculation.
- Topology rerandomisation: the arrangement of each layer of the Mixnet is re-randomised in order to make it more difficult for dishonest nodes to create 'full routes' of dishonest nodes running modified software. Currently, this is also where Nodes may enter or leave the active set based on uptime monitoring.
