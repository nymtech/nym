# Setup & Run

## Viewing command help

You can check that your binaries are properly compiled with:

```
./nym-client --help
```

The two most important commands you will issue to the client are:

* `init` - initalise a new client instance.
* `run` - run a mixnet client process.

You can check the necessary parameters for the available commands by running:

```
./nym-client <command> --help
```

## Initialising your client

Before you can use the client, you need to initalise a new instance of it. Each instance of the client has its own public/private keypair, and connects to its own gateway node. Taken together, these 3 things (public/private keypair + gateway node identity key) make up an app's identity.

Initialising a new client instance can be done with the following command:

```
./nym-client init --id example-client
```

The `--id` in the example above is a local identifier so that you can name your clients; it is **never** transmitted over the network.

There is an optional `--gateway` flag that you can use if you want to use a specific gateway. The supplied argument is the `Identity Key` of the gateway you wish to use, which can be found on the [mixnet explorer](https://explorer.nymtech.net/network-components/). Alternatively, you could use [Harbourmaster](https://harbourmaster.nymtech.net/)

Not passing this argument will randomly select a gateway for your client.

## Running your client
You can run the initalised client by doing this:

```
./nym-client run --id example-client
```

When you run the client, it immediately starts generating (fake) cover traffic and sending it to the mixnet.

When the client is first started, it will reach out to the Nym network's validators, and get a list of available Nym nodes (gateways, mixnodes, and validators). We call this list of nodes the network _topology_. The client does this so that it knows how to connect, register itself with the network, and know which mixnodes it can route Sphinx packets through.
