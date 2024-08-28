# Integrating with Nym for network privacy
If you are wanting to integrate Nym by using the Mixnet as a transport layer for application traffic, you will have to run one of the three Nym clients in order to connect to the Mixnet.

## Connecting applications to the mixnet
### SDK support
If your app is written in Typescript or Rust, then you can use the [Typescript](https://nymtech.net/docs/sdk/typescript.html) or [Rust](https://nymtech.net/docs/sdk/rust.html) SDKs. These SDKs abstract away much of the messaging logic from your app, and allow you to run a Nym client as part of your application process, instead of having to run them seperately.

### Choosing a client
In order to connect your application to the mixnet, you need to select one of three clients to use. These clients do the majority of the heavy-lifting with regards to cryptographic operations and routing under the hood, and all do basically the same thing: create a connection to a gateway, encrypt and decrypt packets sent to and received from the mixnet, and send cover traffic to hide the flow of actual app traffic from observers.

As outlined in the [clients overview documentation](https://nymtech.net/docs/clients/overview.html) there are three clients availiable to developers to use when connecting applications to the mixnet:

#### Websocket client
Your first option is the native websocket client. This is a compiled program that can run on Linux, Mac OS X, and Windows machines. It runs as a persistent process on a desktop or server machine. You can connect to it with any language that supports websockets.

[//]: # (You can see an example of how to connect to and manage interactions with this client in the [Simple Service Provider tutorial]&#40;../tutorials/simple-service-provider/simple-service-provider.md&#41;.)

#### Webassembly client
If you’re working in JavaScript or Typescript in the browser, or building an edge computing app, you’ll likely want to choose the webassembly client.

It’s packaged and available on the npm registry, so you can npm install it into your JavaScript or TypeScript application.

The webassembly client is most easily used via the [typescript sdk](https://nymtech.net/docs/sdk/typescript.html).

You can find example code in the [examples section](https://github.com/nymtech/nym/tree/master/sdk/typescript/examples) of the codebase, and in the [typescript sdk docs](https://nymtech.net/docs/sdk/typescript.html).

#### SOCKS client
This client is useful for allowing existing applications to use the Nym mixnet without any code changes. All that’s necessary is that they can use one of the SOCKS5, SOCKS4a, or SOCKS4 proxy protocols (which many applications can - crypto wallets, browsers, chat applications etc).

It’s less flexible as a way of writing custom applications than the other clients, but able to be used to proxy application traffic through the mixnet without having to make any code changes.

You can find examples of how to utilise this client in the [Quickstart](../quickstart/socks-proxy.md) section, and the [SOCKS5 documentation](https://nymtech.net/docs/clients/socks5-client.html).

## Recommended infrastructure setup
In order to ensure uptime and reliability, it is recommended that you run some pieces of mixnet infrastructure. What infrastructure is necessary to run depends on the architecture of your application, and the endpoints that it needs to hit!

* If you're running a purely P2P application, then just integrating clients and having some method of sharing addresses should be enough to route your traffic through the mixnet.
* If you're wanting to place the mixnet between your users' application instances and a server-based backend, you can use the [network requester](https://nymtech.net/operators/nodes/network-requester-setup.html) service provider binary to proxy these requests to your application backend, with the mixnet 'between' the user and your service, in order to prevent metadata leakage being broadcast to the internet.
* If you're wanting to route RPC requests through the mixnet to a blockchain, you will need to look into setting up some sort of service that does the transaction broadcasting for you. You can find examples of such projects on the [community applications](../community-resources/community-applications-and-guides.md) page.

## Example application traffic flow
### Initialization
First, we need to initalise an app and connect it to Nym.


```
       +-----------+
       |  Gateway  |
       +-----------+
             ^
             |
             |
             |
             |
             |
             |
   +-------------------+
   | +---------------+ |
   | |  Nym client   | |
   | +---------------+ |
   |         ^         |
   |         |         |
   |         |         |
   |         |         |
   |         v         |
   | +---------------+ |
   | | Your app code | |
   | +---------------+ |
   +-------------------+
    Your Local Machine
```

At the bottom we have an app. It consists of two parts:

* your application specific logic
* your Nym client - either running as a standalone process, or as part of the process of your app code if you're using an SDK

Nym apps have a stable, potentially long-lasting relation to a gateway node. A client will register itself with a gateway, and get back an authentication token that it can then use to retrieve messages from the gateway later on.

Gateways serve a few different functions:

* they act as an end-to-end encrypted message store in case your app goes offline.
* they send encrypted surb-acks for potentially offline recipients, to ensure reliable message delivery
* they offer a stable addressing location for apps, although the IP may change frequently

### Sending messages to ourselves
The Nym client part of the app accepts messages from your code and automatically turns it into layer-encrypted Sphinx packets. If your message is too big to fit inside on Sphinx packet, it'll be split into multiple packets with a sequence numbers to ensure reliable automatic reassembly of the full message when it gets to the recipient.

The app has now connected to the Gateway, but we haven't sent a message to ourselves yet. Let's do that now.

```

       +----------+              +----------+             +----------+
       | Mix Node |<-----------> | Mix Node |<----------->| Mix Node |
       | Layer 1  |              | Layer 2  |             | Layer 3  |
       +----------+              +----------+             +----------+
            ^                                                   ^
            |                                                   |
            |<--------------------------------------------------+
            |
            v
    +--------------+
    | Your gateway |
    +--------------+
            ^
            |
            |
            v
  +-------------------+
  | +---------------+ |
  | |  Nym client   | |
  | +---------------+ |
  |         ^         |
  |         |         |
  |         |         |
  |         v         |
  | +---------------+ |
  | | Your app code | |
  | +---------------+ |
  +-------------------+
   Your Local Machine**


** note that depending on the technical setup, the Nym client running on this machine may
be either a seperate process or embedded in the same process as the app code via one of our SDKs.
```

Let's say your code code pokes a message `hello world` into the Nym client. The Nym client automatically wraps that message up into a layer encrypted Sphinx packet, adds some routing information and encryption, and sends it to its own gateway. The gateway strips the first layer of encryption, ending up with the address of the first mixnode it should forward to, and a Sphinx packet.

The gateway forwards the Sphinx packet containing the `hello world` message. Each mixnode in turn forwards to the next mixnode. The last mixnode forwards to the recipient gateway (in this case, our own gateway since we are sending to ourselves).

Our app has presumably not gone offline in the short time since the message was sent. So when the gateway receives the packet, it decrypts the packet and sends the (encrypted) content back to our app.

The Nym client inside the app decrypts the message, and your code receives the message `hello world`, again as a websocket event.

Messages are end-to-end encrypted. Although the gateway knows our app's IP when it connects, it's unable to read any of the message contents.

### Sending messages to other apps
The process for sending messages to other apps is exactly the same, you simply specify a different recipient address. Address discovery happens outside the Nym system: in the case of a Service Provider app, the service provider has presumably advertised its own address. If you're sending to a friend of yours, you'll need to get a hold of their address out of band, maybe through a private messaging app such as Signal.

```

       +----------+              +----------+             +----------+
       | Mix Node |<-----------> | Mix Node |<----------->| Mix Node |
       | Layer 1  |              | Layer 2  |             | Layer 3  |
       +----------+              +----------+             +----------+
            ^                                                   ^
            |                                                   |
            |                                                   |
            v                                                   v
    +--------------+                                   +-----------------+
    | Your gateway |                                   | Service gateway |
    +--------------+                                   +-----------------+
            ^                                                    ^
            |                                                    |
            |                                                    |
            v                                                    v
  +-------------------+                                +-------------------+
  | +---------------+ |                                | +---------------+ |
  | |  Nym client   | |                                | |  Nym Client   | |
  | +---------------+ |                                | +---------------+ |
  |         ^         |                                |         ^         |
  |         |         |                                |         |         |
  |         |         |                                |         |         |
  |         v         |                                |         v         |
  | +---------------+ |                                | +---------------+ |
  | | Your app code | |                                | | Service Code  | |
  | +---------------+ |                                | +---------------+ |
  +-------------------+                                +-------------------+
   Your Local Machine**                               Service Provider Machine**


** note that depending on the technical setup, the Nym client running on these machines may
be either a seperate process or embedded in the same process as the app code via one of our SDKs.
```
