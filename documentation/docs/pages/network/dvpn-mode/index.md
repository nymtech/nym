# dVPN Mode

dVPN mode is a 2-hop decentralized VPN available through [NymVPN](https://nymvpn.com). It provides strong privacy with low latency, suitable for everyday internet use.

## Overview

Unlike traditional VPNs that route traffic through a single provider's server, dVPN mode routes traffic through two independent nodes operated by different parties:

```
┌──────┐         ┌───────────────┐         ┌──────────────┐         ┌──────────┐
│ User │────────▶│ Entry Gateway │────────▶│ Exit Gateway │────────▶│ Internet │
└──────┘         └───────────────┘         └──────────────┘         └──────────┘
     │                   │                        │                       │
     │     WireGuard     │    Layer Encryption    │      Plaintext       │
     │◀─────────────────▶│◀──────────────────────▶│◀─────────────────────▶│
```

## Key Properties

### Decentralization

- Entry and Exit Gateways are operated by independent parties
- No single operator sees both your IP and your destination
- Operators are distributed globally across ~60 countries

### Privacy Guarantees

| Property | Protection |
|----------|------------|
| IP hiding | Your IP is hidden from destination servers |
| Split knowledge | Entry knows your IP; Exit knows destination; neither knows both |
| Packet uniformity | All packets are padded to uniform size |
| Traffic indistinguishability | dVPN traffic looks identical to mixnet traffic |

### Performance

- **Latency**: ~50-150ms additional (comparable to traditional VPNs)
- **Throughput**: High bandwidth suitable for streaming and downloads
- **Protocol**: WireGuard-based for efficient encryption

## How It Works

1. **Connection Setup**
   - Client establishes WireGuard tunnel to Entry Gateway
   - Entry Gateway establishes connection to Exit Gateway
   - zk-nym credential is presented for anonymous authentication

2. **Traffic Flow**
   - Outbound traffic is encrypted and sent to Entry Gateway
   - Entry Gateway re-encrypts and forwards to Exit Gateway
   - Exit Gateway decrypts and forwards to destination
   - Responses follow the reverse path

3. **Packet Processing**
   - All packets are padded to uniform size
   - Layer encryption ensures each hop only sees necessary routing info
   - No timing delays are applied (unlike mixnet mode)

## Security Considerations

### What dVPN Mode Protects Against

- **Destination servers**: Cannot see your real IP
- **Local network observers**: See only encrypted traffic to Entry Gateway
- **Single-node compromise**: No single node has full information
- **ISP logging**: ISP sees only connection to Entry Gateway

### Limitations

dVPN mode does **not** protect against:

- **Global Passive Adversaries**: An entity observing both Entry and Exit can correlate timing
- **Traffic analysis**: Volume and timing patterns are visible
- **Long-term statistical attacks**: Patterns may emerge over extended observation

For protection against these threats, use [Mixnet Mode](../mixnet-mode).

## When to Use dVPN Mode

**Good for:**
- General web browsing
- Streaming video and audio
- File downloads
- Online gaming
- Any activity where speed matters and you face typical adversaries

**Consider Mixnet Mode instead for:**
- Sensitive communications
- High-threat environments
- When timing correlation is a concern
- Maximum privacy requirements

## Comparison with Traditional VPNs

| Aspect | Traditional VPN | Nym dVPN |
|--------|-----------------|----------|
| Trust model | Single provider | Split across two operators |
| Payment privacy | Usually linked | Anonymous via zk-nyms |
| Decentralization | Centralized | Decentralized |
| Packet uniformity | Variable | Uniform size |
| Open source | Varies | Yes |

## Technical Details

For protocol specifications and encryption details, see [dVPN Protocol](./protocol).
