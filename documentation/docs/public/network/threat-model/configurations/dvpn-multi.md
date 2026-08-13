---
title: dVPN · multiple exits
description: Multiple dVPN exits restore request unlinkability at the destination given per-request rotation. Fast, no mixing.
url: https://nym.com/docs/network/threat-model/configurations/dvpn-multi
---

export const scenario = requireGenericScenario('dvpn-multi')

# dVPN · multiple exits

Per-request exit rotation is not available in the Nym SDKs yet. This
configuration is planned for a future release. Until then a dVPN client uses a
fixed exit, which behaves like a single exit at the destination.
