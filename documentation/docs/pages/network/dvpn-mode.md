# dVPN Mode

dVPN mode is a 2-hop decentralized VPN available through [NymVPN](https://nymvpn.com). It provides strong privacy with low latency, suitable for everyday internet use.

## How it works

Unlike traditional VPNs that route traffic through a single provider's server, dVPN mode routes traffic through two independent nodes operated by different parties.

```
User ──▶ Entry Gateway ──▶ Exit Gateway ──▶ Internet
```

Your traffic is encrypted in layers—a tunnel inside a tunnel. The outer layer is encrypted to the Entry Gateway, and the inner layer is encrypted to the Exit Gateway. The Entry Gateway strips the outer layer and forwards the still-encrypted packet. The Exit Gateway strips the inner layer and sends it to the destination. Responses follow the reverse path.

This "onion" model means neither gateway ever sees both your identity and your destination simultaneously. The Entry Gateway knows your IP address but cannot see your destination or message contents. The Exit Gateway knows your destination but cannot see your IP address.

## Privacy guarantees

dVPN mode hides your IP from destination servers and provides decentralization—no single operator sees everything. All packets are padded to uniform size, preventing packet-size fingerprinting. And critically, dVPN traffic is indistinguishable from mixnet traffic to external observers.

The mode does not provide timing obfuscation. Packets are forwarded immediately without delay. It does not generate cover traffic. A sophisticated adversary capable of observing both your Entry and Exit Gateways could potentially correlate timing to link your requests. For protection against such adversaries, use [mixnet mode](/network/mixnet-mode).

## Performance

Latency is typically 50-150ms additional—comparable to traditional VPNs and suitable for real-time activities. Throughput is high enough for streaming and downloads. The WireGuard-based encryption is efficient and handles reconnection gracefully.

## When to use dVPN mode

dVPN mode is appropriate for general web browsing, streaming video and audio, file downloads, and any situation where speed matters and your adversaries are typical—ISPs monitoring your traffic, websites tracking your location, advertisers building profiles.

Consider mixnet mode instead for sensitive communications, journalism, activism, or situations where sophisticated adversaries might be monitoring network traffic.

## Censorship resistance

dVPN mode uses [AmneziaWG](https://docs.amnezia.org/documentation/amnezia-wg/), a fork of WireGuard designed to be harder to detect and block. AmneziaWG introduces decoy packets before the handshake initiation, which can help disrupt some DPI (Deep Packet Inspection) rules used to identify WireGuard traffic.

This is one of several approaches Nym is taking to improve connectivity in restrictive network environments. It's not a guarantee against all blocking methods, but it raises the bar for censors relying on simple protocol fingerprinting.

For more background, see [Introducing AmneziaWG for NymVPN](https://nym.com/blog/introducing-amneziawg-for-nymvpn).

## Technical details

For protocol specifications and encryption details, see [dVPN Protocol](/network/dvpn-mode/protocol).

## Further reading

- [Introducing AmneziaWG for NymVPN](https://nym.com/blog/introducing-amneziawg-for-nymvpn) — censorship resistance
- [What Is a Double VPN?](https://nym.com/blog/double-vpn) — multi-hop privacy explained
- [Building a Decentralized WireGuard VPN](https://nym.com/blog/building-decentralized-wireguard-vpn) — architecture decisions
- [What is NymVPN?](https://nym.com/blog/what-is-nymvpn) — general overview
