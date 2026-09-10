# Add HTTP/2 to the DoH client

## Why

The WASM resolver sends DoH queries (RFC 8484) over the tunnel's `hyper` HTTP/1.1 client. Quad9 retired HTTP/1.1 DoH on 15 December 2025 (https://quad9.net/news/blog/doh-http-1-1-retirement/); its `9.9.9.9/dns-query` endpoint now answers HTTP/1.1 requests with `505 HTTP Version Not Supported`. Quad9 is a default endpoint in `default_doh_endpoints()` and the first fallback after Cloudflare, so every rotation to it wastes a hop on a guaranteed 505.

A connection-probe run (50 establishments, one resolver pinned per run) measured this: Quad9 resolved 0 of 17 attempts, 13 of them `505`, while Cloudflare (15/16) and Google (14/16) worked. The 505s in the logs are all Quad9.

RFC 8484 recommends HTTP/2 as the minimum DoH version. The industry is converging on HTTP/2-only DoH, so Cloudflare and Google may follow Quad9. Adding HTTP/2 to the DoH client fixes Quad9 now and future-proofs the whole DoH path, rather than dropping one resolver.

## What Changes

- The DoH request path negotiates HTTP/2 by ALPN and speaks it when the resolver selects it. A resolver that still offers HTTP/1.1 (Cloudflare, Google today) keeps the existing HTTP/1.1 path unchanged. A resolver that requires HTTP/2 (Quad9) now works instead of returning 505.
- HTTP/2 is confined to the DoH path. The general `mixFetch` path keeps advertising `http/1.1` only, so a normal website can never negotiate HTTP/2 against the HTTP/1.1-only request code. This invariant is load-bearing: if HTTP/2 leaked onto the shared TLS config, a site that negotiated it would break the fetch client in a way that looks like a mixnet fault.
- DoH over HTTP/2 is single-request per connection for now: the connection is not pooled. Session resolutions are already cached (`dns.rs` caches per hostname per session), so the cold handshake is paid once per unique hostname, not per lookup. Warm HTTP/2 session reuse is a named follow-up, not part of this change.
- The whole HTTP/2 DoH path sits behind a `doh-h2` Cargo feature, so its binary-size cost is a clean on/off measurement, a build-graph failure in `h2` on `wasm32-unknown-unknown` cannot block the existing release, and the `dns`-only build does not pay for it unless it opts in.

## Capabilities

### New Capabilities

<!-- none: this change modifies the smol-core-stack capability -->

### Modified Capabilities

- `smol-core-stack`: the DoH resolver path gains HTTP/2 negotiation so resolvers that require HTTP/2 work; the general fetch path is fixed to HTTP/1.1-only as an explicit invariant.

## Impact

- `wasm/smolmix/src/tls.rs`: a second cached `ClientConfig` for DoH advertising `["h2","http/1.1"]`; the general config stays `["http/1.1"]`. A DoH connect returns the negotiated ALPN protocol read inside `tls::connect` before the stream is wrapped.
- `wasm/smolmix/src/http.rs`: a `request_h2` that runs `hyper::client::conn::http2::handshake` with a `spawn_local`-backed `hyper::rt::Executor`, sends one request (absolute URI, minimal headers), reads the response, and does not recover the stream. No h2 keep-alive or timeout options are set, so no `hyper::rt::Timer` impl is needed.
- `wasm/smolmix/src/fetch.rs`: `doh_query` uses the DoH connect; on a negotiated `h2` it calls `request_h2` (unpooled); otherwise the existing pooled HTTP/1.1 path runs unchanged. `new_connection`/`connect_resolved` signatures are untouched, so the general fetch path carries no risk.
- `wasm/smolmix/Cargo.toml`: a `doh-h2` feature enabling `hyper/http2`.
- `wasm/smolmix/tests/tests/connection-probe.spec.mjs`: `classifyDns` records the DoH HTTP status, so a fixed Quad9 reads as `resolved` and any residual failure is distinguishable from the old flat `server-error`.
- Deferred to `design.md` follow-ups: warm HTTP/2 session reuse for DoH, and the resolver canary plus wildcard-endpoint support that would auto-drop a broken resolver.
