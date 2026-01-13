# dVPN Mode

dVPN mode is a 2-hop decentralized VPN available through [NymVPN](https://nymvpn.com). It provides strong privacy with low latency, suitable for everyday internet use.

## How it works

Unlike traditional VPNs that route traffic through a single provider's server, dVPN mode routes traffic through two independent nodes operated by different parties.

```
User ──▶ Entry Gateway ──▶ Exit Gateway ──▶ Internet
```

Your traffic is encrypted and sent to an Entry Gateway, which forwards it to an Exit Gateway, which decrypts and sends it to the destination. Responses follow the reverse path.

The Entry Gateway knows your IP address but cannot see your destination or message contents. The Exit Gateway knows your destination but cannot see your IP address. Neither operator has the complete picture.

## Privacy guarantees

dVPN mode hides your IP from destination servers and provides decentralization—no single operator sees everything. All packets are padded to uniform size, preventing packet-size fingerprinting. And critically, dVPN traffic is indistinguishable from mixnet traffic to external observers.

The mode does not provide timing obfuscation. Packets are forwarded immediately without delay. It does not generate cover traffic. A sophisticated adversary capable of observing both your Entry and Exit Gateways could potentially correlate timing to link your requests. For protection against such adversaries, use [mixnet mode](/network/mixnet-mode).

## Performance

Latency is typically 50-150ms additional—comparable to traditional VPNs and suitable for real-time activities. Throughput is high enough for streaming and downloads. The WireGuard-based encryption is efficient and handles reconnection gracefully.

## When to use dVPN mode

dVPN mode is appropriate for general web browsing, streaming video and audio, file downloads, and any situation where speed matters and your adversaries are typical—ISPs monitoring your traffic, websites tracking your location, advertisers building profiles.

Consider mixnet mode instead for sensitive communications, journalism, activism, or situations where sophisticated adversaries might be monitoring network traffic.

## Technical details

For protocol specifications and encryption details, see [dVPN Protocol](/network/dvpn-mode/protocol).
