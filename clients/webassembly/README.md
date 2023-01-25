# Nym Sphinx webassembly client

Produces layer-encrypted Sphinx packets for use with Nym mixnets. 

[Sphinx](http://www0.cs.ucl.ac.uk/staff/G.Danezis/papers/sphinx-eprint.pdf) packets ensure the privacy of information in transit, even when the adversary is able to monitor the network in its entirety. When used with a mixnet, both content (what you said) and metadata (who you said it to, when you said it) are protected.

This helps browser-based and mobile applications get stronger privacy, in a way that wasn't previously possible.

This client is part of the [Nym](https://nymtech.net/docs) project. It's written in Rust and compiled to WebAssembly.

## Security Status 

From a security point of view, this module is not yet complete. Key missing features include, but are not limited to: cover traffic, sending packets with delay according to Poisson distribution. 

They should be implemented soon. You can build your applications, but don't rely on it for strong anonymity yet if your application needs cover traffic.

## Using it

See the [SDK directory](../../sdk/typescript/examples) for examples on how to use it and the NPM packages available.

## Developing

This is a Rust crate which is set up to automatically cross-compile the contents of `src` to WebAssembly (aka wasm). It's published from the main [Nym platform monorepo](https://github.com/nymtech/nym) in the `clients/webassembly` directory.

First, make sure you've got all the [Rust wasm toolchain](https://rustwasm.github.io/docs/book/) installed. Cross-compilation sounds scary, but the Rust crew have enabled a remarkably simple setup.

Furthermore, [wasm-bindgen documentation](https://rustwasm.github.io/docs/wasm-bindgen/) provides excellent tips to solving common problems.

Whenever you change any Rust in the `src` directory, run `wasm-pack build --scope nymproject` to update the built wasm artefact in the `pkg` directory.

To be clear, this is not something that most JS developers need to worry about, this is only for Nym devs. The packages on NPM have all files in place. Just install and enjoy!

### Packaging

If you're a Nym platform developer who's made changes to the Rust files and wants to re-publish the package to NPM, here's how you do it: 

1. bump version numbers as necessary for SemVer
2. go the `sdk/typescript` directory (off the project root)
3. run:
```
yarn
yarn build
yarn publish
```