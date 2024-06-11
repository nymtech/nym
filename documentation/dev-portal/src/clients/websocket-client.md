# Websocket Client

> The Nym Websocket Client was built in the [building nym](./building-nym.md) section. If you haven't yet built Nym and want to run the code on this page, go there first.

## Current version
```
<!-- cmdrun ../../../../target/release/nym-client --version | grep "Build Version" | cut -b 21-26  -->
```

You can run this client as a standalone process and pipe traffic into it to be sent through the mixnet. This is useful if you're building an application in a language other than Typescript or Rust and cannot utilise one of the SDKs. 

You can find the code for this client [here](https://github.com/nymtech/nym/tree/develop/clients/native). 

