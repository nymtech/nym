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

Generating Documentation
------------------------

You can generate docs by running `npm run docs`. Generated output will appear in the `docs` directory. 

Packaging
------------------------

If you're a Nym platform developer who's made changes to the client and wants to re-publish the package to NPM, here's how you do it: 

1. Bump the version number (use SemVer)
1. `npm run build`
1. `npm login` (if you haven't already)
1. `npm publish`