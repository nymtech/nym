# Two Modes: dVPN and Mixnet

NymVPN provides two distinct privacy modes, each with different tradeoffs between speed and protection level. Both operate on the same Nym Network infrastructure.

## Quick Comparison

| Aspect | dVPN Mode | Mixnet Mode |
|--------|-----------|-------------|
| **Hops** | 2 (Entry → Exit) | 5 (Entry → Mix × 3 → Exit) |
| **Latency** | Low (~50-150ms) | Higher (~200-500ms) |
| **Throughput** | High | Moderate |
| **Timing protection** | Basic | Full |
| **Cover traffic** | No | Yes |
| **Packet mixing** | No | Yes |
| **Use case** | Everyday browsing | High-security needs |

## dVPN Mode (2-Hop)

```
┌────────┐     ┌───────────────┐     ┌──────────────┐     ┌──────────┐
│  User  │────▶│ Entry Gateway │────▶│ Exit Gateway │────▶│ Internet │
└────────┘     └───────────────┘     └──────────────┘     └──────────┘
```

### How It Works

1. Traffic is encrypted and sent to an Entry Gateway
2. The Entry Gateway forwards to an Exit Gateway
3. The Exit Gateway decrypts and sends to the destination
4. Responses follow the reverse path

### Privacy Properties

- **IP hiding**: Your IP is hidden from destination servers
- **Decentralization**: No single VPN provider to trust
- **Encryption**: WireGuard-based encryption between hops
- **Packet padding**: Uniform packet sizes prevent size-based fingerprinting

### Limitations

- **No timing obfuscation**: Packets are forwarded immediately
- **No cover traffic**: Traffic volume is visible to observers
- **Correlation attacks**: A GPA could potentially correlate entry/exit timing

### Best For

- General web browsing
- Streaming video
- Downloads
- Any use case where speed matters and you face typical adversaries (ISPs, websites, advertisers)

## Mixnet Mode (5-Hop)

```
┌────────┐     ┌───────┐     ┌─────────┐     ┌─────────┐     ┌─────────┐     ┌──────┐     ┌──────────┐
│  User  │────▶│ Entry │────▶│ Mix L1  │────▶│ Mix L2  │────▶│ Mix L3  │────▶│ Exit │────▶│ Internet │
└────────┘     └───────┘     └─────────┘     └─────────┘     └─────────┘     └──────┘     └──────────┘
                                  │               │               │
                             Variable         Variable        Variable
                              delay            delay           delay
```

### How It Works

1. Client creates layered Sphinx packets with routing instructions
2. Packets travel through Entry Gateway to three Mix Node layers
3. Each Mix Node:
   - Removes one encryption layer
   - Adds a random delay
   - Forwards to the next hop
4. Exit Gateway sends to destination
5. Cover traffic is continuously generated to mask real traffic

### Privacy Properties

- **Timing obfuscation**: Random delays break timing correlation
- **Traffic analysis resistance**: Cover traffic hides real communication
- **Unlinkability**: Each packet takes an independent random path
- **Unobservability**: Impossible to distinguish real traffic from cover traffic

### Limitations

- **Higher latency**: Mixing delays add ~100-400ms per hop
- **Lower throughput**: Cover traffic consumes bandwidth
- **Message-based**: Works at message level, not raw IP

### Best For

- Sensitive communications
- Whistleblowing or journalism
- Users in high-surveillance environments
- Any scenario where adversaries may be monitoring the network

## Traffic Indistinguishability

A critical design feature: **dVPN and mixnet traffic are indistinguishable to external observers**.

Both modes:
- Use the same Entry and Exit Gateways
- Employ the same encryption standards
- Produce identically-sized packets

This means an observer cannot tell whether a user is in dVPN mode (faster, less protected) or mixnet mode (slower, maximum protection). This ambiguity itself provides privacy benefits.

## Choosing a Mode

**Use dVPN Mode when:**
- Speed is important
- You're accessing streaming services
- Your threat model is typical (hiding from ISPs, websites)
- Convenience matters

**Use Mixnet Mode when:**
- Maximum privacy is required
- You're in a high-risk situation
- Latency is acceptable
- You're concerned about sophisticated adversaries

## SDK Access

Developers using the [Nym SDKs](../../developers) have access to **mixnet mode only**. The dVPN mode is specific to the NymVPN application.

SDK-based applications communicate through the mixnet using the same privacy guarantees as NymVPN's mixnet mode.
