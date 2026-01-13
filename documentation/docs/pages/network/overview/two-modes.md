# Two Modes: dVPN and Mixnet

NymVPN provides two distinct privacy modes. Both operate on the same Nym Network infrastructure, but they make different tradeoffs between speed and protection level.

## dVPN mode

dVPN mode routes traffic through 2 hops—an Entry Gateway and an Exit Gateway. Traffic flows from your device to the Entry Gateway, then to the Exit Gateway, then to the destination.

```
User ──▶ Entry Gateway ──▶ Exit Gateway ──▶ Internet
```

This mode uses WireGuard encryption between you and the Entry Gateway, with additional layer encryption between the gateways. All packets are padded to uniform sizes. Latency is typically 50-150ms additional, comparable to traditional VPNs.

dVPN mode hides your IP from destination servers and splits knowledge between two independent operators—the Entry Gateway knows your IP but not your destination, while the Exit Gateway knows your destination but not your IP. However, it does not add timing delays or cover traffic. A sophisticated adversary observing both gateways could potentially correlate entry and exit timing.

Use dVPN mode for general web browsing, streaming video, downloads, and situations where speed matters and your adversaries are typical—ISPs, websites, advertisers.

## Mixnet mode

Mixnet mode routes traffic through 5 hops—an Entry Gateway, three layers of Mix Nodes, and an Exit Gateway. Each Mix Node adds a random delay and mixes your traffic with other packets passing through.

```
User ──▶ Entry ──▶ Mix L1 ──▶ Mix L2 ──▶ Mix L3 ──▶ Exit ──▶ Internet
                     │           │           │
                  delay       delay       delay
```

Beyond the additional hops, mixnet mode generates constant cover traffic—dummy packets indistinguishable from real ones. This hides not just who you're communicating with, but whether you're communicating at all.

Latency is higher, typically 200-500ms additional, due to the mixing delays. But this mode defeats timing correlation attacks and provides unobservability against even global passive adversaries.

Use mixnet mode for sensitive communications, high-risk situations, journalism, activism, or whenever maximum privacy justifies the latency cost.

## Traffic indistinguishability

A critical design feature: external observers cannot tell whether traffic is using dVPN mode or mixnet mode. Both modes use the same Entry and Exit Gateways, the same packet sizes, and the same encryption. This ambiguity itself provides privacy—observers don't know whether you're in the faster, less protected mode or the slower, maximum protection mode.

## SDK access

Developers using the [Nym SDKs](/developers) have access to mixnet mode only. The dVPN mode is specific to the NymVPN application. SDK-based applications communicate through the mixnet using the same privacy guarantees as NymVPN's mixnet mode.
