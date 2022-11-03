# Mixnet WASM web worker

This directory contains code that must be bundled as a web worker, so there are some restrictions:
- limited options for importing scripts
- `wasm-pack` needs synchronous loading for WASM blobs

# Features

- `comlink` provides messaging wrapper between calling thread and web worker thread
- [worker.ts](./worker.ts) must be bundled as an entry point for the worker
- `URL(..)` types used so the bundler can identify dependent code and bundle correctly

# TODO

- add support for using `Transfer` to move binary objects from the main thread to the web worker
- wire up handler for receiving binary messages