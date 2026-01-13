# Packet Mixing

Packet mixing is the core mechanism that breaks timing correlations between incoming and outgoing traffic at each node.

## The Problem: Timing Correlation

Without mixing, an observer watching a node could:

```
         Observer
            │
            ▼
    ┌───────────────┐
───▶│    Node       │───▶
 t₀ │               │ t₀ + δ
    └───────────────┘

    Input at t₀ correlates with output at t₀ + δ
```

Even with encryption, the timing relationship reveals which input became which output.

## The Solution: Random Delays

Each Mix Node introduces a random delay before forwarding:

```
         Observer
            │
            ▼
    ┌───────────────┐
───▶│    Node       │
 t₀ │   [delay]     │
───▶│   [delay]     │───▶ (could be either input)
 t₁ │   [delay]     │
    └───────────────┘

    Outputs are randomly reordered
```

> Continuous-time mixing strategies ... delay each message independently, forwarding it to its next destination once a specified delay has timed out. The aggregate effect of independently delaying each message is an output sequence of messages that is randomly reordered with respect to the input sequence.
>
> — [Nym Whitepaper](https://nym.com/nym-whitepaper.pdf) §4.4

## Exponential Delay Distribution

Nym uses an **exponential distribution** for delays:

```
Probability
    │
    │█
    │██
    │████
    │███████
    │████████████
    │█████████████████████
    └──────────────────────────▶ Delay time
      Short delays    Long delays
      (common)        (rare but possible)
```

Properties of exponential delays:

- **Memoryless**: No matter how long a packet has waited, the probability of further delay is the same
- **Maximum entropy**: For a given mean delay, exponential distribution maximizes uncertainty
- **Equal probability**: Two packets arriving at different times have equal probability of leaving in either order

## Why Exponential?

> If we observe two messages going into the mix node at times t₀ < t₁, and a message coming out at a later time t₂, the probability that the output is any of the two inputs is equal, regardless of differences in their arrival times t₀ and t₁.
>
> — [Nym Whitepaper](https://nym.com/nym-whitepaper.pdf) §4.4

This is critical: even if an adversary knows exactly when packets arrived, they cannot determine which output corresponds to which input.

## Continuous-Time vs Batch Mixing

### Batch Mixing (e.g., Chaumian mixnets)

```
┌─────────┐     Wait for     ┌─────────┐
│ Collect │────batch fill───▶│ Shuffle │────▶ Release all
│ inputs  │                  │ & send  │
└─────────┘                  └─────────┘
```

Problems:
- Latency is unpredictable (wait for batch to fill)
- Bursty traffic patterns
- Limited anonymity set (just the batch)
- Vulnerable to active attacks

### Continuous-Time Mixing (Nym)

```
    ┌─────────────────────────────────────┐
───▶│ Receive │ Delay │ Send             │───▶
───▶│ Receive │ Delay │ Send             │───▶
───▶│ Receive │ Delay │ Send             │───▶
    └─────────────────────────────────────┘
        Each packet processed independently
```

Advantages:
- **Optimal anonymity** for given mean latency
- **Predictable latency** (mean is configurable)
- **Unbounded anonymity set** (includes past and future packets)
- **Efficient bandwidth** utilization
- **Lower latency** overall

## Implementation

At each Mix Node:

1. **Receive** Sphinx packet
2. **Decrypt** outer layer, verify HMAC
3. **Extract** routing information for next hop
4. **Generate** random delay from exponential distribution
5. **Queue** packet with scheduled send time
6. **Send** packet when delay expires

```
┌────────────────────────────────────────────────────────┐
│                    Mix Node                            │
│                                                        │
│  ┌──────────┐    ┌───────────┐    ┌───────────────┐   │
│  │ Receive  │───▶│  Decrypt  │───▶│  Delay Queue  │   │
│  │          │    │  & Verify │    │  (scheduled)  │   │
│  └──────────┘    └───────────┘    └───────┬───────┘   │
│                                           │           │
│                                           ▼           │
│                                    ┌───────────┐      │
│                                    │   Send    │─────▶│
│                                    └───────────┘      │
└────────────────────────────────────────────────────────┘
```

## Parameters

The mean delay (1/λ) is configurable:
- **Shorter delays**: Lower latency, but less mixing
- **Longer delays**: Better mixing, but higher latency
- **Current default**: Typically 50-100ms mean delay per hop

With 3 mixing layers, total mixing delay averages 150-300ms.

## Anonymity Analysis

The anonymity set grows with:
- More traffic through the node
- Longer mean delays
- More cover traffic

For a given mean delay, exponential distribution provides the **maximum possible anonymity set** among all continuous-time mixing strategies.

## Interaction with Cover Traffic

Mixing works together with [cover traffic](./cover-traffic):
- Cover traffic ensures minimum traffic levels
- Mixing reorders both real and cover packets
- Together they provide both unlinkability and unobservability

## Further Reading

- [Loopix Design](./loopix): Full system design including mixing
- [Nym Whitepaper §4.4](https://nym.com/nym-whitepaper.pdf): Formal analysis of continuous-time mixing
- [Original Loopix Paper](https://arxiv.org/pdf/1703.00536): Academic analysis and proofs
