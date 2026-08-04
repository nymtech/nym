---
title: Traffic-shape hygiene (nym-swizzle)
description: Baseline hygiene shapes the timing and content of requests so the destination learns less from the pattern of what you send. The nym-swizzle library provides the primitives. Experimental, documentation in progress.
url: https://nym.com/docs/developers/swizzle
---

# Traffic-shape hygiene

Baseline hygiene is the second layer of the
[two-layer model](/network/threat-model/two-layer-model): transport-independent
client discipline that shapes the **timing** and **content** of your requests,
so the destination (L2) learns less from the pattern of what you send. It closes
the [V2 timing and V3 content vectors](/network/threat-model/vectors) that
transport alone cannot.

**Experimental — documentation in progress.** The `nym-swizzle` library provides
the traffic-shape primitives (request delay and range). It is unreleased, and
this page is a stub. Full documentation and API tables will land once the crate
is released.
