## Nym Sphinx in WebAssembly

## About

This is an npm package which allows JavaScript programmers (or anyone else who can use WebAssembly in their applications) to produce layer-encrypted [Sphinx](http://www0.cs.ucl.ac.uk/staff/G.Danezis/papers/sphinx-eprint.pdf) packets for use with [Nym](https://nymtech.net/docs) mixnets. It's written in Rust and compiled to WebAssembly.

Sphinx packets are designed to ensure the privacy information in transit, even when the adversary is able to monitor the network in its entirety. When used with a mixnet, both content (what you said) and metadata (who you said it to, when you said it) are protected.

This helps browser-based and mobile applications get stronger privacy, in a way that wasn't previously possible.

## Security Status 

From a security point of view, this module is not yet complete. A key missing feature, cover traffic, will be implemented soon. You can build your applications, but don't rely on it for strong anonymity yet if your application needs cover traffic.

## Using it

See the [Nym docs](https://nymtech.net/docs).

### Demo

There's a demo web application in the `js-example` folder. To run it, first make sure you've got a recent `npm` installed, then follow the instructions in its README.

## Developing

This is a Rust crate which is set up to automatically cross-compile the contents of `src` to WebAssembly (aka wasm). It's published from the main [Nym platform monorepo](https://github.com/nymtech/nym) in the `clients/webassembly` directory.

First, make sure you've got all the [Rust wasm toolchain](https://rustwasm.github.io/book/game-of-life/setup.html) installed. Cross-compilation sounds scary but the Rust crew have enabled a remarkably simple setup.

Whenever you change any Rust in the `src` directory, run `wasm-pack build --scope nymproject` to update the built wasm artefact in the `pkg` directory.

For now, when you compile `nym-client-wasm` using `wasm-pack build --scope nymproject` you will need to manually copy the file `client.js`  into the `pkg` and add it to `package.json`. Once [these](https://github.com/rustwasm/wasm-pack/issues/840) [issues](https://github.com/rustwasm/rfcs/pull/8#issuecomment-564725214) get closed, this annoying extra step will go away.

To be clear, this is not something that most JS developers need to worry about, this is only for Nym devs. The packages on NPM have all files in place. Just install and enjoy!

### Packaging

If you're a Nym platform developer who's made changes to the Rust (or JS) files and wants to re-publish the package to NPM, here's how you do it: 

1. `wasm-pack build --scope nymproject` builds the wasm binaries into the `pkg` directory (not in source control)
2. copy `client.js` into the `pkg` folder and add it to the `package.json` manifest
3. bump version numbers as necessary for SemVer
4. `wasm-pack publish --access=public` will publish your changed package to NPM
