## Nym Sphinx in WebAssembly

This is a Rust crate which is set up to automatically cross-compile the contents of `lib.rs` to WebAssembly (aka wasm).

Wasm is pretty close to bare metal. Browser-based or server-side JavaScript (or other wasm-using environments) can use the wasm output from this crate to create Sphinx packets at much higher speeds than would be possible using (interpreted) JavaScript. This enables browser-based and mobile applications get stronger privacy, in a way that wasn't previously possible.

### Compiling

First, make sure you've got all the [Rust wasm toolchain](https://rustwasm.github.io/book/game-of-life/setup.html) installed. Cross-compilation sounds scary but the Rust crew have enabled a remarkably simple setup.

### Using it as a JavaScripter  

See our [docs](https://nymtech.net/docs).

### Demo

There's a demo web application in the `www` folder. To run it, first make sure you've got a recent `npm` installed, then follow the instructions in its README. 

### Developing

Whenever you change your Rust, run `wasm-pack build` to update the built was artefact in the `pkg` directory.
