# Tasks: Switch MixDNS to DoH

## 1. DoH transport

- [x] 1.1 Add `fetch::doh_query`: DoH GET with base64url (no padding) query in `?dns=`, `accept: application/dns-message`, to `https://<resolver>/dns-query`, pooled with idempotent retry, bounded by the DNS timeout
- [x] 1.2 Route through the tunnel's existing `rustls-rustcrypto` + `hyper` client, not hickory's DoH transport
- [x] 1.3 Reuse `build_query` for the query bytes and `parse_response` for the response body unchanged
- [x] 1.4 Remove the UDP-specific anti-spoof loop (source-address and transaction-id checks); HTTP/TLS request-response pairing supersedes it

## 2. IP-literal TLS

- [x] 2.1 Confirmed no code needed: `ServerName::try_from("1.1.1.1")` yields `ServerName::IpAddress`, rustls verifies the certificate IP SAN via the default webpki verifier
- [x] 2.2 Endpoints are IP-literal HTTPS (Cloudflare 1.1.1.1, Quad9 9.9.9.9, Google 8.8.8.8), all serving IP-SAN certs

## 2a. Feature graph (compiles on wasm32 in every config)

- [x] 2a.1 `dns = ["fetch"]` and `websocket = ["fetch", "dep:async-tungstenite"]` so the HTTP stack is present wherever the always-compiled resolver is; `_http` gains `dep:base64`

## 3. Error visibility

- [x] 3.1 Add `FetchError::DnsRateLimited { endpoint }` and `DnsServerError { endpoint, status }`
- [x] 3.2 Map HTTP 429 to `DnsRateLimited`, other non-success statuses to `DnsServerError`; log both via `console.warn` (unconditional, not debug-gated)
- [x] 3.3 Map SERVFAIL/REFUSED (and other error rcodes) to a distinct server-failure error that rotates to the next endpoint; only NoError/NXDomain go through `parse_response`

## 4. Fast retry and backup endpoint

- [x] 4.1 Drop the per-attempt `dns_timeout` default from 30 s to a cold-TLS-safe budget (8 s); a 429 rotates immediately without consuming it
- [x] 4.2 On 429, server error, or no-response timeout against an endpoint, rotate to the next DoH endpoint; a rate-limit/server error is not masked by an AAAA retry
- [x] 4.3 Reuse the pooled hyper client so only the first lookup pays the cold handshake; later lookups are warm
- [x] 4.4 Keep the endpoint-rotation and A-then-AAAA structure, now bounded by the per-attempt budget

## 5. Configuration

- [x] 5.1 `default_doh_endpoints()`: `https://1.1.1.1/dns-query` (Cloudflare), `https://9.9.9.9/dns-query` (Quad9), `https://8.8.8.8/dns-query` (Google)
- [x] 5.2 `doh_endpoints: Vec<Url>` overridable through `TunnelOpts` (replaces `primary_dns`/`fallback_dns`); builder setter + accessor + JS `SetupOpts.doh_endpoints`

## 6. Preserve existing guarantees

- [x] 6.1 IPv4-only: AAAA still skipped (existing `resolve_with` behaviour)
- [x] 6.2 IP-literal hosts still bypass resolution (existing short-circuit)
- [x] 6.3 CNAME chains still followed up to the existing hop limit
- [x] 6.4 Per-session cache still consulted and populated

## 7. Tests (pending build)

- [ ] 7.1 429 from an endpoint surfaces `DnsRateLimited` and rotates to the next endpoint
- [ ] 7.2 No-response timeout within the 8 s budget rotates to the next endpoint
- [ ] 7.3 A/AAAA and CNAME resolution still return correct addresses over DoH
- [ ] 7.4 SERVFAIL/REFUSED rotate to the next endpoint (server failure, no AAAA retry); NODATA (NoError with empty answers) still returns a no-records error and retries AAAA
- [ ] 7.5 IP-literal host issues no DoH request

## 8. Docs / TS (breaking JS-opts rename)

- [ ] 8.1 Update `mix-tunnel` `SetupMixTunnelOpts` type (`doh_endpoints` replaces `primary_dns`/`fallback_dns`) and note republish
- [ ] 8.2 Update SDK examples, playground, and `mix-dns`/`mix-fetch` docs pages (list in branch scratch file)
- [ ] 8.3 Docs note: DNS is now DoH (encrypted, rate-limit-visible); mixDNS pulls the mixFetch stack; default resolvers + 8 s timeout

## 9. Out of scope

- DoH POST mode; GET is idempotent and reuses the pooled-retry path.
- DoH over QUIC (DoH3) or DoT. The crypto-provider constraint on wasm32 rules out the hickory transports anyway.
- Resolver auto-discovery beyond the overridable endpoint list.
