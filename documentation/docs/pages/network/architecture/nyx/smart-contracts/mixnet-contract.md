# Mixnet Contract

The Mixnet smart contract is a core piece of the Nym system, functioning as the mixnet directory and keeping track of delegations and rewards: the core functionality required by an incentivised mixnet.  You can find the code and build instructions [here](https://github.com/nymtech/nym/tree/master/contracts/mixnet).

> Having a smart contract act as a decentralised topology directory for clients connecting to the Mixnet allows us to mitigate several possible attacks which systems relying on P2P networking are susceptible to. See [Why Nym is not P2P](../../nym-not-p2p).

Functionality
The Mixnet contract has multiple functions:
* storing bonded mix node and gateway information (and removing this on unbonding).
* **providing the network-topology to the (cached) validator API endpoint used by clients on startup for routing information.**
* storing delegation and bond amounts.
* storing reward amounts.

The addresses of deployed smart contracts can be found in the [`network-defaults`](https://github.com/nymtech/nym/blob/master/common/network-defaults/src/mainnet.rs) directory of the codebase alongside other network default values.
