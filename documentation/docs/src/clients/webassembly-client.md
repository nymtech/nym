# Webassembly Client 

The Nym webassembly client allows any webassembly-capable runtime to build and send Sphinx packets to the Nym network, for uses in edge computing and browser-based applications. 

This is currently packaged and distributed for ease of use via the [Nym Typescript SDK library](../sdk/typescript.md). 

The webassembly client allows for the easy creation of Sphinx packets from within mobile apps and browser-based client-side apps (including Electron or similar). 

## Building apps with nym-client-wasm

Check out the [examples section](../sdk/typescript.md#using-the-sdk) of the SDK docs for examples of simple application framework setups. There are also two example applications located in the `clients/webassembly` directory in the main Nym platform codebase. The `js-example` is a simple, bare-bones JavaScript app. 

## Think about what you're sending!
```admonish caution 
Think about what information your app sends. That goes for whatever you put into your Sphinx packet messages as well as what your app's environment may leak.
```

Whenever you write client PEAPPs using HTML/JavaScript, we recommend that you do not load external resources from CDNs. Webapp developers do this all the time, to save load time for common resources, or just for convenience. But when you're writing privacy apps it's better not to make these kinds of requests. Pack everything locally.

If you use only local resources within your Electron app or your browser extensions, explicitly encoding request data in a Sphinx packet does protect you from the normal leakage that gets sent in a browser HTTP request. [There's a lot of stuff that leaks when you make an HTTP request from a browser window](https://panopticlick.eff.org/). Luckily, all that metadata and request leakage doesn't happen in Nym, because you're choosing very explicitly what to encode into Sphinx packets, instead of sending a whole browser environment by default.
