# Tasks: Add HTTP/2 to the DoH client

## 1. Build-graph kill-shot (do first, cheapest)

- [ ] 1.1 Record the baseline release `.wasm` size before any Cargo change, same profile and wasm-opt settings
- [ ] 1.2 Add a `doh-h2` feature to `wasm/smolmix/Cargo.toml` enabling `hyper/http2`
- [ ] 1.3 Add a `spawn_local`-backed `hyper::rt::Executor` and a skeleton `request_h2` behind `doh-h2`
- [ ] 1.4 `cargo check --target wasm32-unknown-unknown --features doh-h2` compiles; if `h2` does not build on wasm32, stop here having spent ~60 lines
- [ ] 1.5 Build with and without `doh-h2` and record the `.wasm` size delta

## 2. TLS ALPN for DoH

- [ ] 2.1 Add a second cached `ClientConfig` for DoH advertising `["h2","http/1.1"]`; leave the general config `["http/1.1"]`
- [ ] 2.2 A DoH connect reads the negotiated ALPN inside `tls::connect` (before wrapping in `MaybeCloseNotify`) and returns it with the stream; no public accessor on `MaybeCloseNotify`
- [ ] 2.3 Reuse the `CONNECT_ATTEMPTS` retry loop for the DoH connect via a small helper; do not change `new_connection`/`connect_resolved` signatures

## 3. HTTP/2 request path

- [ ] 3.1 Complete `request_h2`: absolute URI (h2 needs `:scheme`/`:authority`), method, `Accept: application/dns-message` only; no `Host`, no `Connection` header (both forbidden in h2)
- [ ] 3.2 Spawn the h2 `Connection` future via `spawn_local`, mirroring the HTTP/1.1 driver
- [ ] 3.3 Return `HttpResponse` only (no stream recovery, not poolable)

## 4. Wire DoH to negotiation

- [ ] 4.1 `doh_query` connects via the DoH TLS path; on negotiated `h2` call `request_h2`, else the existing pooled HTTP/1.1 `request`
- [ ] 4.2 Confirm the general fetch path still advertises `http/1.1` only and is unchanged

## 5. Probe verification

- [ ] 5.1 `classifyDns` captures the DoH HTTP status so `resolved` vs a specific error is visible
- [ ] 5.2 Re-run `test:probe`; Quad9 moves from 0/17 to roughly Cloudflare's resolve rate

## 6. Out of scope (captured in design.md)

- Warm HTTP/2 session reuse for DoH (single-request per connection for now)
- Resolver canary and wildcard-endpoint support (auto-drop a broken resolver)
- HTTP/2 on the general `mixFetch` path
