# Nym vs Other Systems

How does the Nym Network compare to other privacy solutions? Each system makes different tradeoffs.

## Nym vs VPNs

Traditional VPNs provide an encrypted tunnel between your device and a VPN server. This hides your IP from destination websites and encrypts traffic from local observers like your ISP.

The fundamental limitation is trust. The VPN provider can see all your traffic—every site you visit, when you visit it, how long you stay. They can log this information voluntarily or be compelled to by legal process. Your payment information directly links to your account and activity.

Nym's dVPN mode improves on this by splitting trust across two independent operators. The Entry Gateway knows your IP but not your destination. The Exit Gateway knows your destination but not your IP. Neither can build a complete picture. Payment is handled through zk-nyms, making subscriptions unlinkable to activity.

For maximum privacy, Nym's mixnet mode goes further with timing obfuscation and cover traffic—protections no traditional VPN offers.

## Nym vs Tor

Tor is the best-known anonymous overlay network. It routes traffic through three relays using onion encryption, where each relay removes one encryption layer. This prevents any single relay from seeing both source and destination.

Tor's design predates the era of practical global passive adversaries. It provides no timing obfuscation—packets flow through without delays. It provides no cover traffic—observers can see when you're communicating and how much. End-to-end timing correlation attacks, once theoretical, are now feasible for well-resourced adversaries.

Nym's mixnet addresses these gaps. Random delays at each mix node break timing correlations. Cover traffic hides when real communication occurs. Per-packet routing (rather than Tor's per-session circuits) prevents long-term route observation. Blockchain-based topology eliminates the centralized directory authority.

Tor may be preferred when you need to access the entire web with lower latency, since Nym's mixnet adds delay. But for message-based communications and scenarios with sophisticated adversaries, the mixnet provides stronger guarantees.

## Nym vs I2P

I2P replaces Tor's directory authority with a distributed hash table. While this improves decentralization, DHT-based routing has known attack vectors. Like Tor, I2P provides no timing protection—packets flow without delays or cover traffic.

Nym's blockchain-based topology is more robust than DHT approaches and provides similar decentralization benefits. The addition of mixing and cover traffic provides protections I2P cannot offer.

## Nym vs end-to-end encryption

End-to-end encryption systems like Signal provide excellent content protection. Messages are encrypted on your device and can only be decrypted by the recipient. The server cannot read message contents.

But E2EE does nothing for metadata. The server sees who you communicate with, when, how often, and how much. This metadata alone can reveal sensitive information about relationships and activities.

Nym and E2EE are complementary. Use E2EE to protect message content. Use Nym to protect metadata. Together they provide comprehensive privacy—neither alone is sufficient.

## Summary

Use Nym dVPN mode when you want decentralized VPN without trusting a single provider, speed matters, and your adversaries are typical. Use Nym mixnet mode when metadata protection is critical and latency is acceptable. Consider Tor for general web browsing with lower latency when mixnet delays are unacceptable. Always combine network-layer protection with end-to-end encryption for defense in depth.

## Further reading

- [What is WireGuard?](https://nym.com/blog/what-is-wireguard-vpn)
- [VPN Tunnels Explained](https://nym.com/blog/vpn-tunnels)
