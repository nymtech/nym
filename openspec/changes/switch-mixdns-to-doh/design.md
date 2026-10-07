# Design: Switch MixDNS to DoH

## Context

`wasm/smolmix/src/dns.rs`, the WASM tunnel-scoped resolver. References are to that file as of this change.

Current transport: a `WasmUdpSocket` (smoltcp UDP over the tunnel) sends a wire-format query to `8.8.8.8:53` (primary) or `1.1.1.1:53` (fallback), then loops on `recv_from` matching the source address and transaction id, bounded by `dns_timeout` (30 s per attempt). Query build (`build_query`) and answer parse (`parse_response`) use hickory-proto wire types.

The failure mode: a rate-limited resolver drops the query. `recv_from` never becomes ready, the timeout runs its full budget, and the primary-then-fallback and A-then-AAAA structure can chain several 30 s stalls. No signal tells the client it is rate-limited.

## Why DoH is the right transport

- Rate-limit visibility. An HTTP response has a status code. `429` is an explicit rate-limit signal that the UDP path cannot carry. This is the main win and the direct answer to the reported silent hang.
- Encryption. The query and answer are protected from the exit IPR and the clearnet path to the resolver.
- Reuse. The tunnel already runs TCP, TLS (`rustls-rustcrypto`), and `hyper` for mix-fetch. DoH is one more HTTPS request over that same stack.

## Why not hickory's DoH transport

hickory-proto 0.26 has DoH, but its DoH and TLS features pull `ring` or `aws-lc-rs` as the crypto provider. Both fail to compile on `wasm32-unknown-unknown`; the only viable provider on that target is `rustls-rustcrypto`. hickory offers no rustcrypto-backed DoH feature. So the DoH request is built by hand on the crate's existing `rustls-rustcrypto` + `hyper` client. This is not a large surface: hickory-proto still builds and parses the DNS wire message; only the carrier changes from a UDP socket to an HTTPS POST.

## Decisions

### GET, not POST

RFC 8484 allows GET (base64url of the query in the `dns` parameter) and POST (raw wire message in the body). GET is chosen: it is idempotent, so it inherits the client's existing pooled-connection retry (POST does not, since a half-sent POST cannot be safely replayed). `accept` is `application/dns-message`; the base64url encoding is no-padding per the RFC.

### IP-literal endpoints and the TLS question, resolved

The resolver must be reachable without first resolving a name, or there is a bootstrap loop. So the endpoints are IP literals: `https://1.1.1.1/dns-query` (Cloudflare), `https://9.9.9.9/dns-query` (Quad9), and `https://8.8.8.8/dns-query` (Google). Quad9 is ahead of Google because it does not log queries and filters known-malicious names, which suits a privacy product.

TLS to an IP literal needs no extra code. The existing `tls::connect` passes the host string to `ServerName::try_from`, which parses `"1.1.1.1"` into `ServerName::IpAddress`; rustls' default webpki verifier then validates the certificate's IP SAN. Cloudflare, Quad9, and Google all serve certificates carrying their resolver IPs in SANs and chaining to a webpki root. No custom verifier, no SNI-hostname pinning, and nothing marked `dangerous`.

### Feature coupling

DoH is the only DNS transport, so the always-compiled resolver now needs the TLS and HTTP stack. `dns` and `websocket` gain a dependency on `fetch` (which aggregates `_tls` + `_http`), and `_http` gains `base64`. This guarantees compilation in every real feature combination and grows the `mixDNS`-only and `websocket`-only builds. That cost was accepted in place of keeping a UDP fallback, which would have doubled the resolver code.

### Connection reuse

A cold TCP+TLS handshake over the mixnet is second-scale. If each DoH query opens a fresh connection, every name costs a full handshake. Reusing the tunnel's pooled `hyper` client keeps the DoH connection warm across queries, so only the first name pays the handshake. Recommended: reuse the existing pooled client rather than a one-shot connection.

### Fast budget and backup

`dns_timeout` default drops from 30 s to 8 s per attempt. The number is set by cold-TLS reality: the first DoH query to a resolver pays a TCP+TLS handshake over the mixnet, measured at roughly 7 s cold (see `reference_mixnet_latency_figures`), so a 3 s budget would fail every first lookup before the handshake finished. 8 s clears the cold handshake.

The cold budget costs nothing in the case the user actually hit. Rate-limiting returns an HTTP `429` immediately, so the resolver rotates to the next endpoint at once, never waiting out the 8 s. The timeout only bites a resolver that black-holes the TLS connection outright, which is rare, and even then the worst case is a few endpoints times 8 s, versus up to a minute of silence today. On a `429` or a 5xx the resolver rotates immediately. The A-then-AAAA and endpoint-rotation structure is kept, now bounded by the per-attempt budget.

### What is preserved and what is dropped

Preserved: SERVFAIL/REFUSED distinct from NXDOMAIN, IPv4-only (AAAA skipped), IP-literal host bypass, CNAME following, the per-session cache.

Dropped: the UDP anti-spoof loop (source-address and transaction-id matching). Over DoH the response is the body of the HTTP response to a specific request over an authenticated TLS channel, so there is no off-path datagram to match or discard. RFC 8484 sets the transaction id to 0 for this reason. The TC (truncation) bit check is also moot over DoH (no 512-byte datagram limit), though keeping a defensive check is harmless.

## Risks

- IP-literal TLS verification (above) is the gating unknown. Everything else reuses existing machinery.
- A resolver that rate-limits with a silent TCP reset rather than a 429 would still need the timeout to fire, but the 3 s budget bounds that to 3 s and the backup retry still runs. DoH cannot be worse than UDP here, and is better whenever the resolver answers 429.

## Non-goals

- DoH GET, DoH3/QUIC, DoT.
- Resolver auto-discovery or arbitrary user resolver hostnames beyond the two overridable endpoints.
