# Nyx Chain Watcher

A simple binary to watch addresses on the Nyx chain and to call webhooks when particular message types are in a block.

Look in [env.rs](./src/env.rs) for the names of environment variables that can be overridden.

## Running locally

Connect with `psql` to your local database:

```sql
CREATE USER nyx_chain_scraper WITH PASSWORD 'scrapymcscrapeface';

CREATE DATABASE nyx_chain_scraper_data;
GRANT ALL ON DATABASE nyx_chain_scraper_data TO nyx_chain_scraper;
```

Then run:

```
cargo run -- init --chain-history-db-connection-string postgres://nyx_chain_scraper:scrapymcscrapeface@localhost/nyx_chain_scraper_data
```

```
NYX_CHAIN_WATCHER_HISTORY_DATABASE_PATH=postgres://nyx_chain_scraper:scrapymcscrapeface@localhost/nyx_chain_scraper_data \
NYX_CHAIN_WATCHER_WATCH_ACCOUNTS=n1...,n1...,n1... \
NYX_CHAIN_WATCHER_WATCH_CHAIN_MESSAGE_TYPES="/cosmos.bank.v1beta1.MsgSend,/ibc.applications.transfer.v1.MsgTransfer"
NYX_CHAIN_WATCHER_WEBHOOK_URL="https://webhook.site" \
NYX_CHAIN_WATCHER_WEBHOOK_AUTH=1234 \
cargo run -- run
```

## sqlx

If you have issues with `sqlx` please see [README_SQLX.md](../README_SQLX.md).


