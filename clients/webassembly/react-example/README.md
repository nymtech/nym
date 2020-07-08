This project was bootstrapped with [Create React App](https://github.com/facebook/create-react-app).

# Nym Sphinx Wasm Demo

This example application demonstrates how to use WebAssembly to create Sphinx packets, in the browser, and forward them to a Nym gateway.

## 🚴 Usage

```
npm install         # set up dependencies
npm run start       # starts a web server at http://localhost:3000
```

Check your dev console for output.

### Rebuild after Rust source changes

Install `wasm-pack`. Instruction are at the [Rust WASM tutorial](https://rustwasm.github.io/docs/book/game-of-life/hello-world.html). 

`wasm-pack build` in the `clients/webassembly` directory (one up) will rebuild the wasm package if you make changes to the Rust source. That will be automatically picked up (and reloaded, if need be) by the npm dev server.