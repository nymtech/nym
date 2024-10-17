# RPC Nodes

RPC Nodes (which might otherwise be referred to as 'Lite Nodes' or just 'Full Nodes') differ from Validators in that they hold a copy of the Nyx blockchain, but do **not** participate in consensus / block-production.

You may want to set up an RPC Node for querying the blockchain, or in order to have an endpoint that your app can use to send transactions.

In order to set up an RPC Node, simply follow the instructions to set up a [Validator]() TODO LINK AFTER MERGE, but **exclude the `nyxd tx staking create-validator` command**.

If you want to fast-sync your node, check out the Polkachu snapshot and their other [resources](https://polkachu.com/seeds/nym).
