# Ephemera - reliable broadcast protocol implementation

Ephemera does reliable broadcast for blocks.

## Short Overview

All Ephemera nodes accept messages submitted by clients. Node then gossips these to other nodes in the cluster. After certain interval,
a node collects messages and produces a block. Then it does reliable broadcast for the block with other nodes in the cluster.

Ephemera doesn't have the concept of (decentralised) leader at the moment. It's up to an _Application_ to decide which block to use. 
For example in case of Nym-Api, it is the first block submitted to a "Smart Contract".

At the same time, the purpose of blocks is to reach consensus about which messages are included. It's just that Ephemera doesn't make the final decision,
instead it leaves that to an _Application_.

## Main concepts

- **Node** - a single instance of Ephemera.
- **Cluster** - a set of nodes participating in reliable broadcast.
- **EphemeraMessage** - a message submitted by a client.
- **Block** - a set of messages collected by a node.
- **Application(ABCI)** - a trait which Ephemera users implement to accept messages and blocks.
  - check_tx
  - check_block
  - accept_block

## How to run

[README](../scripts/README.md)

## HTTP API

See [Rust](src/api/http/mod.rs)

### Endpoints

**NODE**
- `/ephemera/node/health`
- `/ephemera/node/config`

**BLOCKS**
- `/ephemera/broadcast/block/{hash}`
- `/ephemera/broadcast/block/height/{height}`
- `/ephemera/broadcast/blocks/last`
- `/ephemera/broadcast/block/certificates/{hash}`
- `/ephemera/broadcast/block/broadcast_info/{hash}`

**GROUP**
- `/ephemera/broadcast/group/info`

**MESSAGES**
- `/ephemera/broadcast/submit_message`

**DHT**
- `/ephemera/dht/query/{key}`
- `/ephemera/dht/store`

## Rust API

Almost identical to HTTP API.

See [Rust](src/api/mod.rs)

## Application(Ephemera ABCI)

Cosmos style ABCI application hook
- `check_tx`
- `check_block`
- `deliver_block`

See [Rust](src/api/application.rs)

## Examples

### Ephemera HTTP and WS external interfaces example/tests

See [README.md](../examples/http-ws-sync/README.md)

### Nym Api simulation

See [README.md](../examples/nym-api/README.md)

### http API example/tests

See [README.md](../examples/cluster-http-api/README.md)

### Membership over HTTP API example/tests

See [README.md](../examples/members_provider_http/README.md)

## About reliable broadcast and consensus

In blockchain technology blocks have two main purposes:
1. To maintain chain of blocks, so that the validity of each block can be cryptographically verified by the previous blocks
2. As a unit of consensus, each block contains a set of transactions/messages/actions that are agreed upon by
   the network. This set of transactions is chosen from the global set of all possible transactions that are pending.
   We call the set of transactions in a block consensus because the set of nodes trying to achieve global shared state
   agreed on this particular set of transactions.

Ephemera is not a blockchain. But it uses blocks to agree on the set of transactions in a block.
But at the same time it doesn't behave like a blockchain consensus algorithm.
We may say that it allows each application that uses Ephemera to "propose" something what can be
afterwards to be used to achieve consensus.

### In Summary

1. Ephemera provides functionality to reach agreement on a single value between a set of nodes.
2. Ephemera also provides the concept of a block, which application can take advantage of to reach consensus externally.

### Reliable broadcast, consensus and blocks

In distributed systems(including byzantine), we try to solve the problem of reaching to a commons state between a set of nodes.

One way to define this problem is using the following properties:
1.
    1) Agreement: All nodes agree on the same value.(TODO clarify)
    2) Consensus: All nodes agree on the same value.(TODO clarify)
2. Validity: All nodes agree on a value that is valid.
3. Termination: All nodes eventually agree on a value.

Reliable broadcast ensures the properties of 1.1 and 1.2. It's left to a particular consensus algorithm to ensure the termination property.

One important feature of consensus in blockchain is that it guarantees total ordering of transactions.
Reliable broadcast with blocks helps to ensure this total ordering.

### Ephemera specific properties

Because Ephemera doesn't use the idea of leader, we can say that it solves consensus partially.
It allows each instance to create a block. And then it's up to an application to decide which block to use.

Also, as it doesn't implement a full consensus algorithm, it doesn't ensure the termination.
There's no algorithm in place what tries to reach a consensus about a single block globally and sequentially
in time.

When a block contains a single message, then it's semantically equivalent to a reliable broadcast.

But when a block contains multiple messages, then it can be part of a consensus process. Except that in Ephemera each node
can create a block. To achieve consensus in a more traditional sense, it needs an application help if more strict
consensus is required.

For example, Nym-Api allows each node to create a block but uses external coordinator(a smart contract)
to decide which block to use.