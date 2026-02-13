---
title: "Run a Nyx RPC Node for the Nym Network"
description: "Set up and run a dedicated RPC node for the Nyx blockchain. Query network state, serve chain data, and interact with Nym smart contracts programmatically."
schemaType: "HowTo"
section: "Developers"
lastUpdated: "2026-02-01"
---

# RPC Nodes

RPC Nodes (which might otherwise be referred to as 'Lite Nodes' or just 'Full Nodes') differ from Validators in that they hold a copy of the Nyx blockchain, but do **not** participate in consensus / block-production.

You may want to set up an RPC Node for querying the blockchain, or in order to have an endpoint that your app can use to send transactions.

In order to set up an RPC Node, simply follow the instructions to set up a [Validator](../../operators/nodes/validator-setup), but **exclude the `nyxd tx staking create-validator` command**.

If you want to fast-sync your node, check out the Polkachu snapshot and their other [resources](https://polkachu.com/seeds/nym).
