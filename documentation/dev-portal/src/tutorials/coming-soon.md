# Stub: Updates Coming Soon!

There is a lot of development work ongoing with our clients; as such the old tutorials that were here got quite out of date. 

However, you can still access the old [tutorial codebases](https://github.com/nymtech/developer-tutorials) as well as the markdown files in the `tutorial-archives/` directory in the [developer portal docs repo](https://github.com/nymtech/nym/tree/develop/documentation/dev-portal/src/tutorials) if you want. 

More up to date tutorials will be coming soon for using RPC and gRPC, `mixfetch`, as well as using the [FFI libraries](https://github.com/nymtech/nym/tree/develop/sdk/ffi) for interacting with the Mixnet via C++ and Go. 

> Developers who are searching for example code can use the following list as the current 'best practices':
> * Generic traffic transport: the [`zcash-rpc-demo`](https://github.com/nymtech/nym-zcash-rpc-demo) repo, although here used to only pipe RPC traffic, is a proof of concept 'generic' mixnet piping example which exposes a TPC Socket on the client side for incoming traffic, pipes this through the mixnet, and then streams TCP packets 'out' the other side. 
> * In-browser usage: see the [browser examples](../examples/browser-only.md) page for examples using `mixFetch`. 
