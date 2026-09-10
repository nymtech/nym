# Design: HTTP/2 on the DoH path

## Context

The DoH client (`fetch::doh_query` → `http::request`) speaks HTTP/1.1 only. Quad9 retired HTTP/1.1 DoH, so its endpoint returns `505`. HTTP/2 is the fix, but it touches the TLS config, the HTTP client, and the connection model, on `wasm32-unknown-unknown` where the build graph is a known source of surprises.

## Decisions

### D1: HTTP/2 confined to the DoH path

Two TLS `ClientConfig`s: the general one advertises `["http/1.1"]` (unchanged), the DoH one advertises `["h2","http/1.1"]`. Only the DoH request code has an HTTP/2 branch.

Rationale: the general fetch client is HTTP/1.1-only. If the shared config advertised `h2`, a website could negotiate HTTP/2 and `http::request`'s HTTP/1.1 handshake would fail against an HTTP/2 connection, presenting as a random mixnet fault. Confining `h2` to DoH keeps the working fetch path at zero risk. This is a spec requirement, not a comment.

### D2: Single-request per HTTP/2 connection, no pooling (for now)

`dns.rs` caches resolutions per hostname per session, so a cold DoH handshake is paid once per unique hostname, not per lookup. HTTP/2's raw stream cannot be handed back to a fresh HTTP/1.1 handshake, so the existing raw-stream pool does not fit HTTP/2. Rather than build a second pool now, the HTTP/2 DoH connection serves one request and is dropped.

Rationale: the cache already amortises the cost, so warm HTTP/2 reuse is an optimisation, not a correctness need. Deferring it keeps this change to the compatibility fix. If measurement shows the per-hostname cold handshake hurts, the follow-up is small: `dns::resolve` already serialises all DNS, so one `Option<(endpoint, SendRequest)>` slot (SendRequest is Clone, driver stays alive) gives warm reuse without a keyed pool.

### D3: `doh-h2` feature gate

The whole HTTP/2 DoH path is behind `doh-h2` (enables `hyper/http2`). Reasons: the size cost is a clean on/off measurement; a wasm32 build-graph failure in `h2` cannot block the existing release; the `dns`-only TS package does not pay for it. Fits the existing `dns`/`hyper`/`fetch`/`websocket` feature idiom.

Open question for the gate: whether `doh-h2` is default-on for the shipped `mix-fetch` build. That depends on the measured size delta (step 1.5) and is decided after the number is known, not in this change.

## HTTP/2-on-wasm specifics (implementation notes)

These are the things that fail if done the HTTP/1.1 way:

- **Executor.** `http2::handshake(exec, io)` needs `exec: hyper::rt::Executor<F>`. A unit struct whose `execute` calls `wasm_bindgen_futures::spawn_local(fut)` is enough. `spawn_local` does not require `F: Send`, so single-threaded wasm is fine. The `HyperIoAdapter` (already implements `hyper::rt::Read`/`Write`) is reused unchanged.
- **Absolute URI.** The HTTP/2 client synthesises `:scheme` and `:authority` from the request URI, so the request must carry the absolute URL, not the path-only URI the HTTP/1.1 path uses.
- **Forbidden headers.** `Connection: keep-alive` is a connection-specific header banned in HTTP/2 (RFC 9113 §8.2.2), and an explicit `Host` conflicts with `:authority`. The DoH HTTP/2 request carries method, absolute URI, and `Accept: application/dns-message` only.
- **No keep-alive or timeout options.** Setting any `http2::Builder` keep-alive or timeout option makes hyper require a `hyper::rt::Timer` impl too. None are set, so none is needed. The DoH call is already bounded by the outer `dns_timeout`.
- **ALPN readback.** The negotiated protocol is on the rustls `ClientConnection` inside `futures_rustls::client::TlsStream`. `MaybeCloseNotify` hides the inner stream, so the negotiated ALPN is read inside `tls::connect` before wrapping and returned alongside the stream, rather than adding a public accessor.

## Verification

The connection probe is the test: pinning Quad9 per run and re-running `test:probe` should move Quad9 from 0/17 resolved to roughly Cloudflare's rate. `classifyDns` is extended to record the DoH HTTP status so "Quad9 now 200s" is distinguishable from "Quad9 fails differently now" (today both flatten to `server-error`).

## Follow-ups (not this change)

- **Warm HTTP/2 session reuse for DoH** (D2): one `Option<(endpoint, SendRequest)>` slot for the current resolver.
- **Resolver canary + wildcard endpoints.** Let a user point `dohEndpoints` at any public resolver (e.g. from public-dns.info), canary-probe each at setup with a known query, keep the resolvers that answer and drop the rest, logged loudly. This is the general form of the Quad9 lesson: resolver health becomes self-correcting instead of hardcoded. It complements this change (which makes Quad9 answerable) rather than replacing it.
- **HTTP/2 on the general fetch path.** A separate, larger change, gated on measurement rather than assumed. Unlike DoH, no site requires HTTP/2 (HTTP/1.1 is not being retired for general web serving), so the only motive is multiplexing performance, and the payoff over the mixnet is uncertain. HTTP/2 multiplexes every request onto one TCP connection, so a lost segment on a lossy high-RTT path (the mixnet, which already needed a smoltcp RTO bump) head-of-line-blocks all in-flight streams, whereas the current HTTP/1.1 connection-per-origin pool parallelises across independent connections. It would also need a real HTTP/2 session pool (keep `SendRequest` alive and multiplex, with staleness detection) rather than the current raw-stream pool, and it touches the working fetch path (redirects, idempotent retry, credential stripping, close-notify). The gate for that change is a probe measurement: does HTTP/2 multiplexing beat HTTP/1.1 connection-pooling over the mixnet? If not, it is not worth the pool rewrite.
