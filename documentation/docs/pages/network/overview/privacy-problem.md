# The Privacy Problem

## Metadata: The Data About Your Data

When you communicate over the internet, two types of information are transmitted:

1. **Content**: The actual message, file, or data being sent
2. **Metadata**: Information *about* the communication itself

Traditional encryption (TLS, end-to-end encryption) protects content. But metadata remains exposed:

| Metadata Type | What It Reveals |
|---------------|-----------------|
| IP addresses | Who is communicating |
| Timing | When communication occurs |
| Frequency | How often parties communicate |
| Packet sizes | What type of content (video, text, etc.) |
| Connection duration | Length of sessions |

## Why Metadata Matters

> "We kill people based on metadata." — Former NSA Director Michael Hayden

Metadata can reveal:
- **Social graphs**: Who knows whom, organizational structures
- **Behavioral patterns**: Daily routines, habits, interests
- **Sensitive activities**: Medical consultations, legal advice, journalism sources
- **Location history**: Where you've been and when

Even with encrypted content, metadata enables:
- Identification of anonymous users through traffic analysis
- Inference of communication content from patterns
- Construction of detailed profiles over time

## The Adversary Model

The Nym Network is designed to protect against **Global Passive Adversaries (GPAs)**: entities capable of observing traffic across the entire network simultaneously. This includes:

- Nation-state intelligence agencies
- Large corporations with extensive network infrastructure
- ISP-level observers
- Collaborative adversaries sharing data

These adversaries can:
- Monitor all entry and exit points
- Correlate timing across the network
- Apply machine learning to traffic patterns
- Conduct long-term statistical analysis

## Why Traditional Solutions Fall Short

### VPNs
- Single point of trust (the VPN provider sees everything)
- No protection against timing correlation
- Payment information links to usage
- Provider can be compelled to log

### Tor
- Designed before GPA was considered realistic
- No timing obfuscation (vulnerable to end-to-end correlation)
- No cover traffic (traffic patterns are visible)
- Centralized directory authority

### Signal/End-to-End Encryption
- Protects content only
- Metadata fully exposed to servers
- Communication patterns visible to observers

## Nym's Approach

The Nym Network addresses these limitations through:

1. **Decentralization**: No single entity to trust or compromise
2. **Packet mixing**: Reorders traffic to break timing correlation
3. **Cover traffic**: Generates dummy packets indistinguishable from real ones
4. **Uniform packets**: All packets are identical in size
5. **Anonymous credentials**: Payment cannot be linked to usage

The result is a network where observers—even those watching the entire network—cannot determine who is communicating with whom or when real communication is occurring.
