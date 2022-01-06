Nym Validator Client
====================

A TypeScript client for interacting with CosmWasm smart contracts in Nym validators. 

Running tests
-------------

The tests will be separated into three categories: unit, integration and mock.

Currently the command to run all tests: 

```
npm test
```

The tests require `.env.example` being renamed to `.env`. The variables and their values for these tests are currently pointing to the `nym-sandbox` environment.

`Tests are still in development` - the test libary is `jest` and the test script will execute currently with: `--coverage --verbosity false`

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