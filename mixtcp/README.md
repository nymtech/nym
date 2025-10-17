# MixTCP

This is an initial proof of concept of a SmolTCP `device` that uses the Mixnet for transport. It relies on the `IpMixStream` module from the Rust SDK to set up a connection with an Exit Gateway's Ip-Packet-Router, meaning that this is the IP that is seen by the receiver of the request.

This can be used as the basis for building HTTP(S) crates on top of the Mixnet whilst abstracting away the complexities of using the Mixnet for transport.

More to come in the future.

`examples/` contains examples for:
- a TLS ping with Cloudflare
- creating a `reqwest`-like HTTPS `GET` request and receiving a response

## Component Interaction
```sh
                          create_device()
                                |
                 +--------------+---------------+
                 |              |               |
                 v              v               v
           NymIprDevice   NymIprBridge      IpPair
                 |              |            (10.0.x.x)
                 |              |
                 +-- channels --+
                                |
                                v
                           IpMixStream
                                |
                                v
                             Mixnet
```
