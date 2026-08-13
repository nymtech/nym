---
title: Mixnet · rotating exits
description: The mixnet with rotating exits restores request unlinkability at the destination. Strong against the network, still slow.
url: https://nym.com/docs/network/threat-model/configurations/mixnet-rotating
---

export const scenario = requireGenericScenario('mixnet-rotating')

# Mixnet · rotating exits

Per-request exit rotation is not available in the Nym SDKs yet. This
configuration is planned for a future release. Until then a mixnet client uses a
fixed exit, which behaves like a single exit at the destination.
