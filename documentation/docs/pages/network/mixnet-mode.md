# Mixnet Mode

Mixnet mode provides maximum privacy by routing traffic through 5 hops with packet mixing, timing delays, and cover traffic. It is available through [NymVPN](https://nymvpn.com) and the [Nym SDKs](/developers).

## How it works

Traffic passes through five hops: an Entry Gateway, three layers of Mix Nodes, and an Exit Gateway. Each Mix Node adds a random delay before forwarding, mixing your packets with others passing through.

```
User ──▶ Entry ──▶ Mix L1 ──▶ Mix L2 ──▶ Mix L3 ──▶ Exit ──▶ Internet
                     │           │           │
                  delay       delay       delay
```

Beyond the additional hops, mixnet mode generates constant cover traffic—dummy packets indistinguishable from real ones. Your client continuously sends packets into the network whether or not you're actively communicating. Real messages are slotted into this stream of cover traffic.

The client constructs Sphinx packets with layered encryption. Each layer contains routing information for one hop plus the inner encrypted packet. As the packet travels through the network, each node removes its layer to learn the next destination, but cannot see the final destination or payload content.

## Privacy properties

The mixnet provides **unlinkability**. An observer cannot link an incoming packet to an outgoing packet at any node, cannot connect successive packets from the same user, and cannot correlate activity across different sessions.

It provides **unobservability**. An observer cannot determine when real communication is occurring, how much real traffic versus cover traffic is flowing, or whether a particular user is actively communicating.

It defeats **traffic analysis**. Random delays break timing patterns. Cover traffic masks real traffic volume. Uniform packet sizes prevent content-type fingerprinting. Per-packet routing prevents route-based correlation.

## Performance

Latency is higher than dVPN mode, typically 200-500ms additional, due to the mixing delays at each of the three Mix Node layers. This is the cost of timing obfuscation. For most messaging applications, this latency is acceptable. For real-time applications like video calls, dVPN mode may be more appropriate.

## When to use mixnet mode

Use mixnet mode for sensitive communications where metadata protection matters—journalism, activism, whistleblowing, or any situation where sophisticated adversaries might be monitoring network traffic. The latency cost is worthwhile when the privacy benefit is critical.

For general browsing and streaming where speed matters more than maximum privacy, consider [dVPN mode](/network/dvpn-mode).

## Further reading

The following pages cover mixnet internals in detail:

- [Loopix Design](/network/mixnet-mode/loopix) explains the academic foundation
- [Traffic Flow](/network/mixnet-mode/traffic-flow) shows the packet journey with diagrams
- [Cover Traffic](/network/mixnet-mode/cover-traffic) explains how dummy packets provide unobservability
- [Packet Mixing](/network/mixnet-mode/mixing) covers timing delays and their importance
- [Anonymous Replies](/network/mixnet-mode/anonymous-replies) describes SURBs for bidirectional communication
