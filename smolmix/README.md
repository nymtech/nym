# SmolMix

This is an initial proof of concept of a SmolTCP `device` that uses the Mixnet for transport. It relies on the `IpMixStream` module from the Rust SDK to set up a connection with an Exit Gateway's Ip-Packet-Router, meaning that this is the IP that is seen by the receiver of the request.

This can be used as the basis for building more generic transport crates on top of the Mixnet (e.g. trying to mirror the interface of a common HTTPS crate) whilst abstracting away the complexities of using the Mixnet for transport.

More to come in the future.

`examples/` contains examples for:
- `cloudflare_ping` - HTTPS request to Cloudflare through the mixnet
- `https_client` - `reqwest`-like HTTPS `GET` client with timed clearnet comparison
- `tls` - TLS handshake diagnostics with state logging
- `dns_udp` - DNS A-record lookup over UDP with timed clearnet comparison

## Component Interaction
```sh
                              create_device()
                                    |
                 +----------+-------+-------+-----------+
                 |          |               |           |
                 v          v               v           v
           NymIprDevice  NymIprBridge  ShutdownHandle  IpPair
                 |          |               |        (10.0.x.x)
                 |          |               |
                 +- channels +    shutdown signal
                                |
                                v
                           IpMixStream
                                |
                                v
                             Mixnet
```
