# Nym Sphinx Wasm Demo

This example application demonstrates how to use WebAssembly to create Sphinx packets, in the browser, and forward them to a Nym gateway.

## 🚴 Usage

```
npm run start # fires up a web page at http://localhost:8001, check your dev console for output.
```

### Rebuild after Rust source changes

`./wasm-pack build` in the `nym-sphinx-wasm` directory (one up) will rebuild the wasm package if you make changes to the Rust source. That will be automatically picked up (and reloaded, if need be) by the npm dev server.

### How was this built? 

This is a skeleton demo app, built from a template at: https://github.com/rustwasm/create-wasm-app

See the [Rust WASM tutorial](https://rustwasm.github.io/docs/book/game-of-life/hello-world.html) for more. This was generated using the command `npm init wasm-app www` in case you'd like to see where it came from originally.

### License

Licensed under

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)

