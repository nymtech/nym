---
title: "dVPN Mode"
description: "How Nym's decentralized VPN mode routes traffic through two independent gateways, splitting trust so no single operator sees both your identity and destination."
schemaType: "TechArticle"
section: "Network"
lastUpdated: "2026-03-15"
---

# dVPN Mode

dVPN mode is a 2-hop decentralized VPN available through [NymVPN](https://nymvpn.com) — traffic is routed through two independent gateways rather than a single VPN provider's server, so no single operator ever sees both who you are and what you're doing.

## How it works

```
User --> Entry Gateway --> Exit Gateway --> Internet
```

Your device constructs a layered encryption envelope — a tunnel inside a tunnel. The outer layer is encrypted to the Entry Gateway and the inner layer to the Exit Gateway. The Entry Gateway strips the outer layer and forwards the still-encrypted packet; the Exit Gateway strips the inner layer and sends the plaintext request to the destination. Responses follow the reverse path. This means neither gateway ever sees both your identity and your destination simultaneously — the Entry Gateway knows your IP address but cannot see where your traffic is going, while the Exit Gateway knows the destination but has no way to determine who sent the request.

## Privacy guarantees

dVPN mode hides your IP from destination servers and splits trust across two operators, but it does not add timing obfuscation or cover traffic — packets are forwarded immediately without delay, which means a sophisticated adversary observing both your Entry and Exit Gateways could correlate timing to link your requests. For protection against that kind of adversary, see [Mixnet Mode](/network/mixnet-mode).

## Performance

Latency is typically 50-150ms additional, comparable to traditional VPNs, since WireGuard handles encryption and reconnection without much overhead.

For help deciding between dVPN and Mixnet mode, see [Choosing a Mode](/network/overview/choosing-a-mode).

## Technical details

- [dVPN Protocol](/network/dvpn-mode/protocol) — protocol stack and encryption details
- [Censorship Resistance](/network/dvpn-mode/censorship-resistance) — AmneziaWG and DPI evasion

## Further reading

- [Introducing AmneziaWG for NymVPN](https://nym.com/blog/introducing-amneziawg-for-nymvpn) — censorship resistance
- [What Is a Double VPN?](https://nym.com/blog/double-vpn) — multi-hop privacy explained
- [Building a Decentralized WireGuard VPN](https://nym.com/blog/building-decentralized-wireguard-vpn) — architecture decisions
- [What is NymVPN?](https://nym.com/blog/what-is-nymvpn) — general overview
