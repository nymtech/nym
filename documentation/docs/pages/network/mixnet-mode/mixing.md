# Packet Mixing

Packet mixing breaks timing correlations by adding random delays at each Mix Node. It's the core mechanism that prevents traffic analysis.

## The problem

Without mixing, an observer watching a node could correlate inputs and outputs. A packet arriving at time t₀ and a packet leaving at time t₀ + δ are obviously related. Even with encryption hiding contents, the timing relationship reveals which input became which output.

## The solution

Each Mix Node adds a random delay before forwarding. Packets don't flow through in order—they're held for variable times and released in a different sequence than they arrived. An observer sees packets going in and packets coming out, but cannot match them.

```
Input sequence:  A B C D E
                 | | | | |
                 v v v v v
              [   Mixing   ]
                 | | | | |
                 v v v v v
Output sequence: C A E B D
```

The delays follow an exponential distribution. This choice is mathematically optimal: if two packets arrive at times t₀ and t₁, they have equal probability of leaving in either order, regardless of when they arrived. The adversary gains no information from timing observations.

## Why exponential delays

The exponential distribution is "memoryless"—the probability of a packet leaving in the next moment doesn't depend on how long it's already waited. This means the adversary cannot narrow down possibilities by noting how long packets have been in the node.

Any other delay distribution leaks information. Fixed delays would let adversaries match arrivals to departures by timing. Uniform distributions would create windows where matches become more likely. The exponential distribution maximizes uncertainty.

## Continuous vs batch mixing

Older mixnet designs collected packets into batches and shuffled them before release. This has problems: latency is unpredictable since you wait for batches to fill, bandwidth is inefficient due to bursty traffic, and the anonymity set is limited to the batch size.

Continuous-time mixing processes each packet independently. Latency is predictable (the mean delay is configurable). Bandwidth is used efficiently. And the anonymity set is unbounded—it includes all packets that have ever passed through, weighted by time.

## The aggregate effect

With three Mix Node layers, each applying random delays, the overall effect is thorough reordering. Packets entering the mixnet in sequence exit in a completely different order. The timing relationship between sending and receiving is destroyed.

This is why mixnet mode has higher latency than dVPN mode. The delays are the price of timing protection. Mean delays of 50-100ms per hop add up to 150-300ms average across three layers—noticeable, but worth it for the privacy gain.

## Combined with cover traffic

Mixing and cover traffic work together. Cover traffic ensures there's always packets to mix, even during low activity. Mixing ensures that real and cover packets become interleaved and indistinguishable. Neither mechanism alone is sufficient—together they provide both unlinkability and unobservability.
