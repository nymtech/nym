---
title: Packet Mixing
description: How the mixnet delays and reorders fixed-size packets across three mix layers, adding a per-layer random delay and cover traffic so output order cannot be tied to input order.
url: https://nym.com/docs/network/deep-dives/mixing
---

export const MixnetDeepDive = dynamic(
  () => import('../../../components/threat-model/MixnetDeepDive').then((m) => m.MixnetDeepDive),
  { ssr: false },
)

# Packet Mixing

Mixing is what separates the mixnet from a fast tunnel. Each packet takes an independent random route through three layers of mix nodes. Every mix node then holds each packet for an independent random delay (the Poisson mixing of Loopix) before forwarding it. Because delays are independent, the order packets leave a layer no longer matches the order they arrived, so an observer cannot follow a packet through. Cover traffic fills the gaps so the real packets are not the only thing moving.

The cost is latency. Every hop adds geographic propagation, and each of the three mix layers adds its own mixing delay on top. Use the controls to expand the mix layers, change how many nodes sit in each layer, and see the latency budget update. Switch to the dVPN contrast to see the same path with the mixing removed: straight through, order and timing preserved.

## Related

- [Mixnet mode: mixing](/network/mixnet-mode/mixing) for the conceptual overview.
- [Packet anatomy](/network/deep-dives/packet-anatomy): how a message becomes the fixed-size packets that get mixed here.
- [dVPN cover and crowding](/network/deep-dives/dvpn-cover): the contrasting mode with no mixing delay.
- [Threat actors](/network/threat-model/actors) and the [two-layer model](/network/threat-model/two-layer-model) for what mixing does and does not defend against.
