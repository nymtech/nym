---
title: "dVPN Mode"
description: "How Nym's decentralized VPN mode routes traffic through two independent gateways, splitting trust so no single operator sees both your identity and destination."
schemaType: "TechArticle"
section: "Network"
lastUpdated: "2026-03-15"
---

# dVPN Mode

dVPN mode is a 2-hop decentralized VPN available through [NymVPN](https://nymvpn.com). Traffic is routed through two independent gateways rather than a single VPN provider's server, so no single operator ever sees both who you are and what you're doing.

## How it works

```
User --> Entry Gateway --> Exit Gateway --> Internet
```

Your device wraps each packet in two layers of encryption, one per gateway. The Entry Gateway strips the outer layer and forwards a packet it cannot read; the Exit Gateway strips the inner layer and sends the plaintext request to the destination. Responses follow the reverse path. The Entry Gateway therefore knows your IP address but not the destination, while the Exit Gateway knows the destination but not the sender.

## Privacy guarantees

dVPN mode hides your IP from destination servers and splits trust across two operators. It does not add timing obfuscation or cover traffic. Packets are forwarded immediately, so an adversary watching both gateways could still correlate timing to link your requests. If you need protection against traffic analysis, see [Mixnet Mode](/network/mixnet-mode).

## Performance

Added latency is comparable to traditional VPNs, and WireGuard keeps cryptographic overhead low, so browsing, streaming, and downloads are not noticeably affected.

## Technical details

- [dVPN Protocol](/network/dvpn-mode/protocol): protocol stack and encryption details
- [Censorship Resistance](/network/dvpn-mode/censorship-resistance): AmneziaWG and DPI evasion

## Further reading

- [Introducing AmneziaWG for NymVPN](https://nym.com/blog/introducing-amneziawg-for-nymvpn): censorship resistance
- [What Is a Double VPN?](https://nym.com/blog/double-vpn): multi-hop privacy explained
- [Building a Decentralized WireGuard VPN](https://nym.com/blog/building-decentralized-wireguard-vpn): architecture decisions
- [What is NymVPN?](https://nym.com/blog/what-is-nymvpn): general overview
