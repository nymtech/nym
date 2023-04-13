# Typescript SDK
The Typescript SDK allows developers to start building browser-based mixnet applications quickly, by simply importing the SDK into their code via NPM as they would any other Typescript library.

You can find the source code [here](https://github.com/nymtech/nym/tree/release/{{platform_release_version}}/sdk) and the library on NPM [here](https://www.npmjs.com/package/@nymproject/sdk).

Currently developers can use the SDK to do the following **entirely in the browser**:
* Create a client
* Listen for incoming messages and reply to them
* Encrypt text and binary-encoded messages as Sphinx packets and send these through the mixnet

> We will be fleshing out further mixnet-related features in the coming weeks with functionality such as importing/exporting keypairs for developing apps with a retained identity over time.

In the future the SDK will be made up of several components, each of which will allow developers to interact with different parts of Nym's infrastructure.

| Component | Functionality                                                                  | Released |
| --------- | ------------------------------------------------------------------------------ | -------- |
| Mixnet    | Create clients & keypairs, subscribe to Mixnet events, send & receive messages | ✔️       |
| Coconut   | Create & verify Coconut credentials                                            | ❌       |
| Validator | Sign & broadcast Nyx blockchain transactions, query the blockchain             | ❌       |

### How it works
The SDK can be thought of as a 'wrapper' around the compiled [WebAssembly client](https://github.com/nymtech/nym/tree/release/{{platform_release_version}}/clients/webassembly) code: it runs the client (a Wasm blob) in a web worker. This allows us to keep the work done by the client - such as the heavy lifting of creating and multiply-encrypting Sphinx packets - in a seperate thread from our UI, enabling you to build reactive frontends without worrying about the work done under the hood by the client eating your processing power.

The SDK exposes an interface that allows developers to interact with the Wasm blob inside the webworker from frontend code.

### Framework Support
Currently, the SDK **only** works with frameworks that use either `Webpack` or `Parcel` as bundlers. If you want to use the SDK with a framework that isn't on this list, such as Angular, or NodeJS, **here be dragons!** These frameworks will probably use a different bundler and you will probably run into problems.

| Bundler | Supported |
| ------- | --------- |
| Webpack | ✔️        |
| Packer  | ✔️        |

Support for environments with different bundlers will be added in subsequent releases.

| Environment      | Supported |
| ---------------- | --------- |
| Browser          |  ✔️       |
| Headless NodeJS  |  ❌       |
| Electron Desktop |  ❌       |


### Using the SDK
There are multiple example projects in [`nym/sdk/typescript/examples/`](https://github.com/nymtech/nym/tree/release/{{platform_release_version}}/sdk/typescript/examples/), each for a different frontend framework.

#### Vanilla HTML
The best place to start if you just want to quickly get a basic frontend up and running with which to experiment is `examples/plain-html`:

```typescript
{{#include ../../../../sdk/typescript/examples/plain-html/src/index.ts}}
```

As you can see, all that is required to create an ephemeral keypair and connect to the mixnet is creating a client and then subscribing to the mixnet events coming down the websocket, and adding logic to deal with them.

#### Parcel
If you don't want to use `Webpack` as your app bundler, we have an example with `Parcel` located at [`examples/parcel/`](https://github.com/nymtech/nym/tree/release/{{platform_release_version}}/sdk/typescript/examples/react-webpack-with-theme-example/parcel/).

#### Create React App
For React developers we have an example which is a basic React app scaffold with the additional logic for creating a client and subscribing to mixnet events in [`examples/react-webpack-with-theme-example/`](https://github.com/nymtech/nym/tree/release/{{platform_release_version}}/sdk/typescript/examples/react-webpack-with-theme-example/).

### Developers: think about what you're sending (and importing)!
Think about what information your app sends. That goes for whatever you put into your Sphinx packet messages as well as what your app's environment may leak.

Whenever you write client PEApps using HTML/JavaScript, we recommend that you **do not load external resources from CDNs**. Webapp developers do this all the time, to save load time for common resources, or just for convenience. But when you're writing privacy apps it's better not to make these kinds of requests. **Pack everything locally**.

If you use only local resources within your Electron app or your browser extensions, explicitly encoding request data in a Sphinx packet does protect you from the normal leakage that gets sent in a browser HTTP request. [There's a lot of stuff that leaks when you make an HTTP request from a browser window](https://panopticlick.eff.org/). Luckily, all that metadata and request leakage doesn't happen in Nym, because you're choosing very explicitly what to encode into Sphinx packets, instead of sending a whole browser environment by default.
