Nym Validator
=============

The Nym Validator has several jobs: 

* use Tendermint (v0.33.0) to maintain a total global ordering of incoming transactions
* track quality of service for mixnet nodes (mixmining)
* generate Coconut credentials and ensure they're not double spent
* maintain a decentralized directory of all Nym nodes that have staked into the system
  
Some of these functions may be moved away to their own node types in the future, for example to increase scalability or performance. At the moment, we'd like to keep deployments simple, so they're all in the validator node.

Running the validator
---------------------

1. Download and install [Tendermint 0.32.7](https://github.com/tendermint/tendermint/releases/tag/v0.32.7)
2. `tendermint init`
3. `tendermint node`