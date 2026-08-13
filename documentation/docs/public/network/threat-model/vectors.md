---
title: Linkage vectors
description: The three vectors that let an adversary attribute or link requests: session state (V1), timing (V2), and content (V3). What each consists of, which actors observe it, and the transport and hygiene countermeasures that close it.
url: https://nym.com/docs/network/threat-model/vectors
---

# Linkage vectors

A vector is a channel through which an adversary attributes a request to a user,
or links two requests to each other. There are three. Each vector lists what it
consists of, which actors can observe it, and the countermeasures that close it.
Every countermeasure belongs to one of the two layers: transport (Layer 1) or
baseline hygiene (Layer 2).

The layer tags map each countermeasure to
[the two-layer model](/network/threat-model/two-layer-model). Layer 1 is what
the transport chooses; Layer 2 is client discipline you owe regardless of
transport.
