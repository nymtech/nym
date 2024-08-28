# Importing and using a custom network topology
If you want to send traffic through a sub-set of nodes (for instance, ones you control, or a small test setup) when developing, debugging, or performing research, you will need to import these nodes as a custom network topology, instead of grabbing it from the [`Mainnet Nym-API`](https://validator.nymtech.net/api/swagger/index.html).


There are two ways to do this:

## Custom Topology Provider
If you are also running a Validator and Nym API for your network, you can specify that endpoint. Clients will then use this endpoint to grab a network topology on startup. You can also use this to specify using a testnet.

## Import a specific topology manually
If you aren't running a Validator and Nym API, and just want to import a specific sub-set of mix nodes, you can also overwrite the grabbed topology manually.
