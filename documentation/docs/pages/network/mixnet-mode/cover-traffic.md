---
title: "Cover Traffic"
description: "How constant dummy packet streams hide real communication patterns in the Nym mixnet, achieving unobservability even against global network observers."
schemaType: "TechArticle"
section: "Network"
lastUpdated: "2026-03-15"
---

# Cover Traffic

Cover traffic consists of dummy packets that hide when real communication is occurring, providing unobservability: an adversary cannot determine whether a user is actively communicating.

## The problem

Even with perfect encryption and mixing, traffic analysis can reveal information. An adversary can see how much data you're sending, when you're sending it, and detect patterns over time. Regular silence followed by bursts of activity reveals your schedule. Consistent traffic volumes to certain destinations reveal ongoing relationships.

## The solution

Cover traffic maintains a constant rate of packet transmission. When you have real data to send, it replaces a cover packet in the stream. When you have nothing to send, cover packets flow anyway. To an observer, the traffic looks identical either way.

```
Without cover traffic:
              |         |||        |
Time ---------+---------+++---------+------>
            Idle    Activity    Idle
                    (visible)

With cover traffic:
    ||||||||||||||||||||||||||||||||||||||
Time -------------------------------------->
         Constant rate (activity hidden)
```

The cover packets are real Sphinx packets with valid encryption, just with empty payloads. They travel through the network exactly like real packets, get mixed at each hop, and are discarded at their destination. No node along the way can tell whether a packet contains real data or is cover traffic.

## Loop traffic

Cover packets follow complete routes through the network back to the sender. These "loops" serve multiple purposes: they provide traffic for mixing with others' cover traffic and they can detect active attacks. If loop packets stop returning, a network fault or active interference is likely.

Mix nodes also generate their own cover traffic, ensuring minimum traffic levels even when few users are active. This provides baseline anonymity guarantees regardless of network load.

## How it's generated

Traffic follows a Poisson process with a configurable rate parameter. Inter-packet times are exponentially distributed: random, but with a known average rate. This distribution provides maximum entropy (uncertainty) for a given mean rate, which translates to optimal privacy properties.

## Tradeoffs

More cover traffic provides better unobservability but uses more bandwidth and, when zk-nyms are enabled, more credential value. Less cover traffic reduces costs but may allow some inference about activity patterns.

The default parameters balance privacy and resource usage. Applications with heightened privacy requirements can increase the cover traffic rate; applications where unobservability is less critical can reduce it.

## What cover traffic defeats

Cover traffic prevents volume analysis (how much you communicate), timing analysis (when you communicate), and behavioral profiling (your communication patterns over time). Combined with packet mixing, it ensures that even an adversary watching the entire network cannot learn about your communication behavior with currently known methods.
