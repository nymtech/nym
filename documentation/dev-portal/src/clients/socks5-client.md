# Socks5 Client

> The Nym socks5 client was built in the [building nym](./building-nym.md) section. If you haven't yet built Nym and want to run the code on this page, go there first.

## Current version
```
<!-- cmdrun ../../../../target/release/nym-socks5-client --version | grep "Build Version" | cut -b 21-26  -->
```

## What is this client for?
Many existing applications are able to use either the SOCKS4, SOCKS4A, or SOCKS5 proxy protocols. If you want to send such an application's traffic through the mixnet, you can use the `nym-socks5-client` to bounce network traffic through the Nym network, like this:

```
                                                                              External Systems:
                                                                                     +--------------------+
                                                                             |------>| Monero blockchain  |
                                                                             |       +--------------------+
                                                                             |       +--------------------+
                                                                             |------>|    Email server    |
                                                                             |       +--------------------+
                                                                             |       +--------------------+
                                                                             |------>|    RPC endpoint    |
                                                                             |       +--------------------+
                                                                             |       +--------------------+
                                                                             |------>|       Website      |
                                                                             |       +--------------------+
                                                                             |       +--------------------+
  +----------------------------------+                                       |------>|       etc...       |
  | Mixnet:                          |                                       |       +--------------------+
  |       * Gateway your client is   |                                       |
  |       connected to               |          +--------------------+       |
  |       * Mix nodes 1 -> 3         |<-------->| Network requester  |<------+
  |       * Gateway that network     |          +--------------------+
  |       requester is connected to  |
  +----------------------------------+
           ^
           |
           |
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
 |         |         |
 |         v         |
 | +---------------+ |
 | | Your app code | |
 | +---------------+ |
 +-------------------+
  Your Local Machine
```

There are 2 pieces of software that work together to send SOCKS traffic through the mixnet: the `nym-socks5-client`, and the `nym-network-requester`.

The `nym-socks5-client` allows you to do the following from your local machine:
* Take a TCP data stream from a application that can send traffic via SOCKS5.
* Chop up the TCP stream into multiple Sphinx packets, assigning sequence numbers to them, while leaving the TCP connection open for more data
* Send the Sphinx packets through the mixnet to a [network requester](https://nymtech.net/operators/nodes/network-requester.md). Packets are shuffled and mixed as they transit the mixnet.

The `nym-network-requester` then reassembles the original TCP stream using the packets' sequence numbers, and make the intended request. It will then chop up the response into Sphinx packets and send them back through the mixnet to your  `nym-socks5-client`. The application will then receive its data, without even noticing that it wasn't talking to a "normal" SOCKS5 proxy!

