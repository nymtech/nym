# Nym vs Other Systems

How does the Nym Network compare to other privacy solutions?

## Comparison Summary

| Feature | VPNs | Tor | I2P | Nym |
|---------|------|-----|-----|-----|
| IP hiding | Yes | Yes | Yes | Yes |
| Decentralized | No | Partial | Yes | Yes |
| Timing obfuscation | No | No | No | Yes (mixnet) |
| Cover traffic | No | No | No | Yes (mixnet) |
| GPA resistance | No | No | No | Yes |
| Incentivized | Some | No | No | Yes |
| Anonymous payment | Rarely | N/A | N/A | Yes |

## Nym vs VPNs

**Traditional VPNs** provide an encrypted tunnel between your device and a VPN server.

### Limitations of VPNs

- **Single point of trust**: The VPN provider can see all your traffic
- **Logging risks**: Provider can be compelled to log or may log voluntarily
- **Payment linkage**: Your subscription is linked to your activity
- **No timing protection**: Traffic patterns are visible
- **Centralized**: Single company controls the service

### Nym's Advantages

- Decentralized operation—no single entity to trust
- Payment unlinkable to usage via zk-nyms
- Multiple hops prevent any single node from seeing full picture
- Mixnet mode provides timing obfuscation
- No central authority to compel logging

## Nym vs Tor

**Tor** is an onion routing network using three-hop circuits.

### How Tor Works

- Traffic is encrypted in layers (like an onion)
- Each relay removes one encryption layer
- Three relays: Guard → Middle → Exit
- Circuits are long-lived (minutes)

### Tor's Limitations

- **No timing obfuscation**: Packets are forwarded immediately
- **No cover traffic**: Traffic patterns are observable
- **Vulnerable to GPA**: End-to-end timing correlation is feasible
- **Centralized directory**: Directory authorities are a trust point
- **No economic incentives**: Relies on volunteers

### Nym's Advantages

- Continuous-time mixing with random delays
- Cover traffic provides unobservability
- Per-packet routing (no long-lived circuits)
- Decentralized topology via blockchain
- Economic incentives ensure network health

### When Tor May Be Preferred

- Accessing the entire web (Nym mixnet is message-based)
- Lower latency requirements
- Established ecosystem and tooling

## Nym vs I2P

**I2P** (Invisible Internet Project) is a peer-to-peer anonymous network.

### How I2P Works

- Uses "garlic routing" (bundling messages)
- Distributed hash table for routing
- Primarily for accessing I2P internal services
- All participants are also routers

### I2P's Limitations

- **DHT vulnerabilities**: Distributed hash tables have known attack vectors
- **Security by obscurity**: Assumes adversaries can't observe full network
- **No timing protection**: Like Tor, packets flow without delays
- **Complex setup**: Not user-friendly

### Nym's Advantages

- Blockchain-based topology eliminates DHT attacks
- Designed to resist global passive adversaries
- Cover traffic and mixing provide stronger guarantees
- Simpler user experience via NymVPN

## Nym vs Signal/E2EE

**End-to-end encryption** (E2EE) like Signal protects message content.

### What E2EE Protects

- Message contents are encrypted client-to-client
- Server cannot read messages
- Strong cryptographic guarantees

### What E2EE Doesn't Protect

- **Who** is communicating
- **When** communication occurs
- **How often** parties communicate
- **Message sizes** and patterns

### Nym's Complement

Nym operates at the network layer, protecting metadata that E2EE cannot. They are complementary:

- Use E2EE for content protection
- Use Nym for metadata protection
- Together, they provide comprehensive privacy

## Summary: When to Use Nym

**Use Nym (dVPN mode) when:**
- You want decentralized VPN without trusting a single provider
- Speed matters but you want better privacy than traditional VPNs
- Anonymous payment is important

**Use Nym (mixnet mode) when:**
- You face sophisticated adversaries
- Metadata protection is critical
- You're willing to accept higher latency for maximum privacy

**Consider Tor when:**
- You need to access the full web with lower latency
- Mixnet latency is unacceptable
- You're using Tor-specific services (.onion)

**Use alongside E2EE:**
- Nym protects the network layer
- E2EE protects message contents
- Both together provide defense in depth
