# Censorship Resistance

dVPN mode incorporates several techniques to help users connect in restrictive network environments where VPN protocols are actively detected and blocked.

## The problem: protocol fingerprinting

Deep Packet Inspection (DPI) systems deployed by ISPs and governments can identify VPN protocols by their handshake patterns, packet sizes, and timing characteristics. Standard WireGuard, for instance, has a recognisable handshake initiation pattern that DPI rules can match against. Once identified, connections can be throttled or blocked entirely.

This is not a theoretical concern — countries including China, Russia, Iran, and others actively deploy DPI to restrict VPN usage.

## AmneziaWG

dVPN mode uses [AmneziaWG](https://docs.amnezia.org/documentation/amnezia-wg/), a fork of WireGuard that adds obfuscation techniques to make the protocol harder to fingerprint.

AmneziaWG modifies the WireGuard handshake by introducing decoy packets before the handshake initiation. These decoy packets disrupt DPI rules that rely on matching the standard WireGuard handshake sequence. The actual WireGuard protocol behaviour is preserved — the modifications sit around the handshake rather than replacing it, so all of WireGuard's security properties (Curve25519 key exchange, ChaCha20-Poly1305 encryption, forward secrecy) remain intact.

AmneziaWG implementation: [`nym-vpn-core/crates/nym-wg-go`](https://github.com/nymtech/nym-vpn-client/tree/main/nym-vpn-core/crates/nym-wg-go)

## Limitations

AmneziaWG raises the bar for censors relying on simple protocol fingerprinting, but it doesn't help against deeper analysis — statistical fingerprinting of packet timing and sizes, IP-based blocking of known Gateway addresses, or active probing where the censor sends packets to suspected VPN servers to confirm their identity.

## QUIC transport mode

QUIC transport mode wraps the WireGuard/AmneziaWG connection inside a [QUIC](https://datatracker.ietf.org/doc/html/rfc9000) layer, so the traffic looks like standard HTTPS/HTTP3 to DPI systems rather than a VPN tunnel. Since QUIC is now used by a significant portion of regular web traffic (over 30% of Cloudflare's traffic in 2023 was HTTP/3 over QUIC), blocking it outright would break large parts of the web for everyone, making it an unattractive target for censors.

QUIC transport applies to the Entry Gateway connection only (the first hop). Not all Gateways support it yet — enabling QUIC in the NymVPN app will filter the Gateway list to those that do. Because the QUIC wrapper adds overhead, it can reduce speeds slightly, so it's worth leaving disabled unless you're in a censored environment or having connectivity issues.

## Stealth API Connect

Even if a user can establish a VPN tunnel, censors can also block access to the API that the NymVPN app needs to discover Gateways and fetch network state in the first place. Stealth API Connect addresses this by routing the app's API requests through a mechanism that is harder to identify and block, so the app can bootstrap its connection to the Nym network even in environments where the Nym API endpoints are actively censored.

## Limitations

These techniques are layered — AmneziaWG obfuscates the handshake, QUIC disguises the tunnel as regular web traffic, and Stealth API Connect protects the initial API discovery. Together they cover several common censorship methods, but none of them are guarantees. Censorship resistance is an ongoing arms race, and new techniques will be documented here as they ship.

## Further reading

- [Introducing AmneziaWG for NymVPN](https://nym.com/blog/introducing-amneziawg-for-nymvpn)
- [AmneziaWG documentation](https://docs.amnezia.org/documentation/amnezia-wg/)
- [What is QUIC? Censorship-Resistant Internet Connections](https://nym.com/blog/what-is-quic)
- [What is QUIC transport mode in NymVPN?](https://support.nym.com/hc/en-us/articles/39648047741457-What-is-QUIC-transport-mode-in-NymVPN)
- [What is Stealth API Connect in NymVPN?](https://support.nym.com/hc/en-us/articles/39652289741329-What-is-Stealth-API-connect-in-NymVPN)
- [NymVPN's roadmap for censorship resistance](https://nym.com/blog/NymVPN-Roadmap-for-censorship-resistance-2025)
