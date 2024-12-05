# Nyx Chain Watcher

A simple binary to watch addresses on the Nyx chain and to call webhooks when particular message types are in a block.

## Running locally

```
DATABASE_URL=nyx_chain_watcher.sqlite \
NYXD_WEBSOCKET_URL=wss://rpc.nymtech.net:443/websocket \
NYXD_RPC_URL=https://rpc.nymtech.net \
PAYMENT_RECEIVE_ADDRESS=n1... \
WEBHOOK_URL=https://webhook.site/... \
cargo run
```

