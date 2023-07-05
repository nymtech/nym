# Nym Firefox Extension Example

This is an example of how Nym can be used within the context of a Mozilla Firefox extension.

## Running the example

Copy a build of the Nym TypeScript SDK (ESM version) into `./sdk`.

Then, Open `sdk/index.js` and change the following line:
```js
var WorkerFactory = createURLWorkerFactory('web-worker-0.js');
```

to:

```js
var WorkerFactory = createURLWorkerFactory('sdk/web-worker-0.js');
```

The above annoying workaround is currently necessary for Firefox extensions.

Load the extension normally via manifest.json.

## How does it work?

The Nym Mixnet Client runs a [Web Worker](https://developer.mozilla.org/en-US/docs/Web/API/Web_Workers_API) that wraps
a WASM library that builds and encrypts Sphinx packets in the browser to send over the Nym mixnet:

![Sphinx packet](../docs/worker.svg)

The WASM code encrypts each layer of the Sphinx packet in the browser, before sending the Sphinx packet over a websocket to the ingress gateway:

![Sphinx packet](../docs/sphinx.svg)
