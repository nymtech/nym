# Clients Overview

A large proportion of the Nym mixnet's functionality is implemented client-side. 

Clients perform the following actions on behalf of users: 

* determine network topology - what mixnodes exist, what their keys are, etc.
* register with a gateway
* authenticate with a gateway
* receive and decrypt messages from the gateway
* create layer-encrypted Sphinx packets
* send Sphinx packets with real messages
* send Sphinx packet _cover traffic_ when no real messages are being sent
* retransmit un-acknowledged packet sends - if a client sends 100 packets to a gateway, but only receives an acknowledgement ('ack') for 95 of them, it will resend those 5 packets to the gateway again, to make sure that all packets are received.  

## Types of Nym clients
At present, there are three Nym clients:

- the websocket (native) client
- the SOCKS5 client
- the wasm (webassembly) client

You need to choose which one you want incorporate into your app. Which one you use will depend largely on your preferred programming style and the purpose of your app.

### The websocket client
Your first option is the native websocket client (`nym-client`). This is a compiled program that can run on Linux, Mac OS X, and Windows machines. It can be run as a persistent process on a desktop or server machine. You can connect to it with **any language that supports websockets**. 

_Rust developers can import websocket client functionality into their code via the [Rust SDK](../sdk/rust/rust.md)_. 

### The webassembly client
If you're working in JavaScript or Typescript in the browser, or building an [edge computing](https://en.wikipedia.org/wiki/Edge_computing) app, you'll likely want to choose the webassembly client. 

It's packaged and [available on the npm registry](https://www.npmjs.com/package/@nymproject/nym-client-wasm), so you can `npm install` it into your JavaScript or TypeScript application. 

_The webassembly client is most easily used via the [Typescript SDK](../sdk/typescript.md)_. 

### The SOCKS5 client
The `nym-socks5-client` is useful for allowing existing applications to use the Nym mixnet without any code changes. All that's necessary is that they can use one of the SOCKS5, SOCKS4a, or SOCKS4 proxy protocols (which many applications can - crypto wallets, browsers, chat applications etc). 

When used as a standalone client, it's less flexible as a way of writing custom applications than the other clients, but able to be used to proxy application traffic through the mixnet without having to make any code changes. 

_Rust developers can import socks client functionality into their code via the [Rust SDK](../sdk/rust/rust.md)_.

## Commonalities between clients
All Nym client packages present basically the same capabilities to the privacy application developer. They need to run as a persistent process in order to stay connected and ready to receive any incoming messages from their gateway nodes. They register and authenticate to gateways, and encrypt Sphinx packets.
