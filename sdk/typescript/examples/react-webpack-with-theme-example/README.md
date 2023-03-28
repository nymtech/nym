# React Nym Mixnet Chat App

This is an example of using the Nym Mixnet to send text chat messages, with optional binary file attachments.

You can use this example as a seed for a new project, and it uses:

- React
- Webpack
- WASM
- Web Workers
- Material UI (MUI)

## Running the example

Try out the chat app by running:

```
npm install
npm start
```

## How does it work?

The Nym Mixnet Client runs a [Web Worker](https://developer.mozilla.org/en-US/docs/Web/API/Web_Workers_API) that wraps 
a WASM library that builds and encrypts Sphinx packets in the browser to send over the Nym mixnet:

![Web worker](../docs/worker.svg)

The chat app has a custom payload that sends either string messages or binary files:

![Packet format](../docs/packet_format.svg)

String messages are sent like this:

![Packet format - string](../docs/packet_string.svg)

Files include the filename and mime-type, so that the recipient can download them with the correct name:

![Packet format - binary](../docs/packet_jpeg.svg)

The WASM code encrypts each layer of the Sphinx packet in the browser, before sending the Sphinx packet over a websocket to the ingress gateway:

![Sphinx packet](../docs/sphinx.svg)




