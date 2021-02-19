Nym Validator Client
====================

A TypeScript client for interacting with CosmWasm smart contracts in Nym validators. 

Running examples
-----------------

With the code checked out, `cd examples`. This folder contains runnable example code that will set up a blockchain and allow you to interact with it through the client. 

Running tests
-------------

```
npm test
```

You can also trigger test execution with a test watcher. I don't have the centuries of life left to me that are needed to fight through the arcana of wiring up a working TypeScript mocha triggered execution setup, so for now my Cargo-based hack is:


```
cargo watch -s "cd clients/validator && npm test"
```

It's ugly but works fine if you have Cargo installed. TypeScript setup help happily accepted here. 