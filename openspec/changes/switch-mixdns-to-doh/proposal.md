# Switch MixDNS to DoH

## Why

The WASM resolver (`wasm/smolmix/src/dns.rs`) sends plain unencrypted DNS over UDP port 53 through the mixnet tunnel: primary `8.8.8.8:53`, fallback `1.1.1.1:53`. Two problems follow from the UDP transport.

First, a rate-limited resolver drops the query rather than answering it. UDP DNS carries no rate-limit signal, so the client cannot tell a drop from a slow answer. With `dns_timeout` at 30 seconds per attempt, and the primary-then-fallback and A-then-AAAA structure, a black-holed name produces up to a minute of silent dead air with no diagnosis. This is the reported silent hang.

Second, the queries and answers are unencrypted on the path from the exit IPR to the public resolver, so the exit and the clearnet path between it and the resolver can read and tamper with them.

Switching the transport to DNS-over-HTTPS (DoH, RFC 8484) fixes both. An HTTP response carries a status code, so a `429 Too Many Requests` becomes an explicit, reportable condition instead of a silent drop. The query and answer are encrypted end to end to the resolver. DoH also rides the TCP and TLS path the tunnel already runs for mix-fetch, so it reuses existing machinery rather than adding a new one.

Note on the original framing: the resolver is not DoT today, it is plain UDP:53. The switch is UDP to DoH, which adds encryption where there is none and, more importantly, gives the rate-limit visibility the UDP path cannot.

## What Changes

- Replace the UDP:53 transport in `resolve`/`query_record` with a DoH GET (RFC 8484 §4.1): the existing hickory-proto wire-format query is base64url-encoded (no padding) into the `dns` query parameter of an HTTP GET to `https://<resolver>/dns-query`, and the response body is parsed by the existing `parse_response`. Only the transport changes; query building and answer parsing are reused. GET is chosen over POST because it is idempotent, so it inherits the client's existing pooled-connection retry for free.
- The DoH request runs over the tunnel's existing TCP, TLS, and HTTP client stack (`rustls-rustcrypto` plus `hyper`), the same path mix-fetch already uses. It does not use hickory's own DoH transport, which pulls `ring` or `aws-lc-rs`, both of which fail to compile on `wasm32-unknown-unknown`. TLS to an IP-literal resolver needs no extra code: `ServerName::try_from` yields an `IpAddress` and rustls verifies the certificate's IP SAN.
- Because DoH is the only DNS transport, the DNS resolver now depends on the TLS and HTTP stack. The `dns` and `websocket` features gain a dependency on `fetch`, so the standalone `mixDNS` export and `wss://` hostname resolution both pull the HTTP stack. This grows those builds; it is the accepted cost of making DNS visible and encrypted, and no UDP fallback is kept.
- Rate-limit and server errors become visible: HTTP `429` maps to a distinct `DnsRateLimited` error and other non-success statuses map to a `DnsServerError(status)` error, both logged. A rate-limited resolver is reported, not black-holed.
- Faster retry with backup endpoints: the per-attempt DNS timeout drops from 30 seconds to a cold-TLS-safe budget (default 8 seconds). The common failure, rate-limiting, returns an HTTP `429` immediately, so rotation to the next endpoint is instant and never waits out the timeout. The timeout only applies to a resolver that black-holes the TLS connection outright, which is rare. The fast path is narrower than a blanket claim: a repeat lookup of a cached hostname makes no query at all (the DNS cache is keyed on the exact hostname), and a lookup that reuses a pooled HTTP/1.1 connection skips the handshake, but the HTTP/2 DoH path is single-request and returns no poolable connection, so a lookup of a different uncached hostname still pays a fresh TCP and TLS handshake.
- Endpoints are IP-literal HTTPS URLs so the resolver itself needs no prior DNS: primary `https://9.9.9.9/dns-query` (Quad9, no-logging and malware-filtering), then `https://1.1.1.1/dns-query` (Cloudflare), then `https://8.8.8.8/dns-query` (Google). All remain overridable through `TunnelOpts`. Quad9 as the primary fits a privacy product better than defaulting resolution to a logging resolver.
- Existing resolver guarantees are preserved where they still apply: SERVFAIL and REFUSED stay distinct from NXDOMAIN and empty results, the stack stays IPv4-only (AAAA skipped), IP-literal hosts still bypass resolution, and CNAME chains are still followed. The DNS transaction-id anti-spoof check is superseded by HTTP request and response pairing over TLS (RFC 8484 sets the id to 0), so it is dropped on the DoH path.

## Capabilities

### New Capabilities

<!-- none: this change modifies the smol-core-stack capability -->

### Modified Capabilities

- `smol-core-stack`: the tunnel-scoped DNS resolver moves from plain UDP:53 to DoH over the tunnel's TLS/HTTP stack, gains explicit rate-limit and server-error reporting, and gains a fast per-attempt timeout with quick backup-endpoint retry.

## Impact

- `wasm/smolmix/src/dns.rs`: `query_record` (the UDP send/recv loop) becomes a DoH GET via `fetch::doh_query`. The UDP-specific anti-spoof loop (source and transaction-id checks) is removed; `build_query` and `parse_response` are reused. `DEFAULT_PRIMARY_DNS`/`DEFAULT_FALLBACK_DNS` become `default_doh_endpoints()` (Quad9, Cloudflare, Google). `resolve` loops the endpoint list.
- `wasm/smolmix/src/fetch.rs`: new `doh_query` helper (GET, base64url `?dns=`, pooled with idempotent retry, bounded by `dns_timeout`).
- `wasm/smolmix/src/tunnel.rs`: `primary_dns`/`fallback_dns` (SocketAddr) collapse to `doh_endpoints: Vec<Url>`; `dns_timeout` default 30s to 8s; accessor `doh_endpoints()`.
- `wasm/smolmix/src/lib.rs`: JS `SetupOpts.primary_dns`/`fallback_dns` become `doh_endpoints: Option<Vec<String>>` (breaking JS-opts change).
- `wasm/smolmix/Cargo.toml`: `dns = ["fetch"]`, `websocket = ["fetch", ...]`, `_http` gains `base64`.
- New error variants on `FetchError`: `DnsRateLimited { endpoint }` and `DnsServerError { endpoint, status }`, logged unconditionally via `console.warn`.
- Docs and TS: the `doh_endpoints` rename is breaking for the published `mix-tunnel` `SetupMixTunnelOpts` type; SDK examples, playground, and `mix-dns` docs need updating (tracked in the branch scratch file).
