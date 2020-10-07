Nym Validator
=============

The Nym Validator has several jobs: 

* use Tendermint (v0.33.0) to maintain a total global ordering of incoming transactions
* rewards + stake slashing based quality of service  measurements for mixnet nodes (aka "mixmining")
* generate Coconut credentials and ensure they're not double spent
* maintain a decentralized directory of all Nym nodes that have staked into the system

Some of these functions may be moved away to their own node types in the future, for example to increase scalability or performance. At the moment, we'd like to keep deployments simple, so they're all in the validator node.
