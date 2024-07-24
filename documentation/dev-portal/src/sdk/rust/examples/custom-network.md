# Importing and using a custom network topology
If you want to send traffic through a sub-set of nodes (for instance, ones you control, or a small test setup) when developing, debugging, or performing research, you will need to import these nodes as a custom network topology, instead of grabbing it from the [`Mainnet Nym-API`](https://validator.nymtech.net/api/swagger/index.html) (`examples/custom_topology_provider.rs`).

There are two ways to do this:

## Import a custom Nym API endpoint
If you are also running a Validator and Nym API for your network, you can specify that endpoint as such and interact with it as clients usually do (under the hood):

```rust,noplayground
{{#include ../../../../../../sdk/rust/nym-sdk/examples/custom_topology_provider.rs}}
```

## Import a specific topology manually
If you aren't running a Validator and Nym API, and just want to import a specific sub-set of mix nodes, you can simply overwrite the grabbed topology manually:

```rust,noplayground
{{#include ../../../../../../sdk/rust/nym-sdk/examples/manually_overwrite_topology.rs}}
```
