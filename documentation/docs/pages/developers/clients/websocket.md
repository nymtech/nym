# Websocket Client

You can run this client as a standalone process and pipe traffic into it to be sent through the mixnet. This is useful if you're building an application in a language other than Typescript or Rust and cannot utilise one of the SDKs.

You can find the code for this client [here](https://github.com/nymtech/nym/tree/master/clients/native).

## Download or compile client

If you are using OSX or a Debian-based operating system, you can download the `nym-socks5-client` binary from our [Github releases page](https://github.com/nymtech/nym/releases).

If you are using a different operating system, or want to build from source, simply use `cargo build --release` from the root of the Nym monorepo.
