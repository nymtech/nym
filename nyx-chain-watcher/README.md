# Nyx Chain Watcher

A simple binary to watch addresses on the Nyx chain and to call webhooks when particular message types are in a block.

Look in [env.rs](./src/env.rs) for the names of environment variables that can be overridden.

## Running locally

```
NYX_CHAIN_WATCHER_HISTORY_DATABASE_PATH=chain_history.sqlite \
NYX_CHAIN_WATCHER_DATABASE_PATH=nyx_chain_watcher.sqlite \
NYX_CHAIN_WATCHER_WATCH_ACCOUNTS=n1...,n1...,n1... \
NYX_CHAIN_WATCHER_WEBHOOK_URL="https://webhook.site" \
NYX_CHAIN_WATCHER_WEBHOOK_AUTH=1234 \
cargo run -- run
```


