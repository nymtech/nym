# Mixnet Mode

Mixnet mode routes traffic through 5 hops — an Entry Gateway, three layers of Mix Nodes, and an Exit Gateway — with random delays, packet reordering, and cover traffic at each mixing layer. It is available through [NymVPN](https://nymvpn.com) and the [Nym SDKs](/developers).

## How it works

Traffic passes through five hops: an Entry Gateway, three layers of Mix Nodes, and an Exit Gateway. Each Mix Node adds a random delay before forwarding, mixing your packets with others passing through.

```
User --> Entry --> Mix L1 --> Mix L2 --> Mix L3 --> Exit --> Internet
                    |           |           |
                  delay       delay       delay
```

Beyond the additional hops, mixnet mode generates constant cover traffic—dummy packets indistinguishable from real ones. Your client continuously sends packets into the network whether or not you're actively communicating. Real messages are slotted into this stream of cover traffic.

The client constructs Sphinx packets with layered encryption. Each layer contains routing information for one hop plus the inner encrypted packet. As the packet travels through the network, each node removes its layer to learn the next destination, but cannot see the final destination or payload content.

## Privacy properties

The combination of mixing, delays, and cover traffic gives the mixnet three properties that simpler systems like VPNs and Tor don't have:

- **Unlinkability**: an observer watching a Mix Node cannot correlate incoming packets with outgoing ones, cannot connect successive packets from the same user, and cannot link activity across different sessions — the random delays and reordering destroy the timing signal that makes this possible in other networks.
- **Unobservability**: because your client sends a constant stream of cover traffic whether or not you're actually communicating, an observer cannot tell when real communication is occurring, how much of the traffic is real versus dummy, or even whether a given user is active at all.
- **Resistance to traffic analysis**: uniform Sphinx packet sizes prevent content-type fingerprinting, per-packet routing means there are no long-lived circuits to observe (unlike Tor), and the mixing delays mean that even an adversary watching the entire network cannot correlate entry and exit timing.

## Performance

Latency is higher than dVPN mode, typically 200-500ms additional, due to the mixing delays at each of the three Mix Node layers. This is the cost of timing obfuscation. For most messaging applications, this latency is acceptable. For real-time applications like video calls, dVPN mode may be more appropriate.

For help deciding between dVPN and Mixnet mode, see [Choosing a Mode](/network/overview/choosing-a-mode).

## Further reading

The following pages cover mixnet internals in detail:

- [Loopix Design](/network/mixnet-mode/loopix) explains the academic foundation
- [Traffic Flow](/network/mixnet-mode/traffic-flow) shows the packet journey with diagrams
- [Cover Traffic](/network/mixnet-mode/cover-traffic) explains how dummy packets provide unobservability
- [Packet Mixing](/network/mixnet-mode/mixing) covers timing delays and their importance
- [Anonymous Replies](/network/mixnet-mode/anonymous-replies) describes SURBs for bidirectional communication
