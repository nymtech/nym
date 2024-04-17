# Nym Node

```admonish note
If you are a `nym-mixnode` or `nym-gateway` operator and you are not familiar wwith the binary changes called *Project Smoosh*, you can read the archived [Smoosh FAQ](../archive/smoosh-faq.md) page.
```

NYM NODE is a tool for running a node within the Nym network. Nym Nodes containing functionality such as `mixnode`, `entry-gateway` and `exit-gateway` are fundamental components of Nym Mixnet architecture. Nym Nodes are ran by decentralised node operators.

To setup any type of Nym Node, start with either building [Nym's platform](../binaries/building-nym.md) from source or download [pre-compiled binaries](../binaries/pre-built-binaries.md) on the [configured server (VPS)](vps-setup.md) where you want to run the node. Nym Node will need to be bond to [Nym's wallet](wallet-preparation.md). Follow [preliminary steps](preliminary-steps.md) page before you initialise and run a node.

```admonish info
**Migrating an existing node to a new `nym-node` is simple. The steps are documented on the [next page](setup.md#migrate)**
```

## Steps for Nym Node Operators

Once VPS and Nym wallet are configured, binaries ready, the operators of `nym-node` need to:

**1. [Setup & Run](setup.md) the node**
**2.** (Optional but reccomended) **[Configure](configuration.md) the node** (WSS, reversed proxy, automation)
**3. [Bond](bonding.md) the node to the Nym API,** using Nym wallet
