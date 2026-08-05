# Tasks: fix-coderabbit-findings-6953

## 1. Correctness

- [x] 1.1 `nym-sdk-session/src/session.rs`: stop racing the cancellation token around the ticket-spending registration exchange in `register_single_hop`/`register_two_hop`/`register_two_hop_quic`. Check cancellation up front (and after gateway selection), then run `register_*_inner`'s LP exchange to completion without a `select!` against cancel. (session.rs:460)
- [x] 1.2 `common/smol-core/src/dns.rs`: in `query_one`, after id/source validation, reject a response with the `TC` (truncated) bit set — return an error instead of extracting partial answers. Add a distinct error if warranted. (dns.rs)
- [x] 1.3 `sdk/rust/smol-dvpn/src/bridge.rs`: wrap `conn.open_bi()` in the same `CONNECT_TIMEOUT` + cancellation `select!` as `transport_conn()`. (bridge.rs:141)

## 2. Robustness / cleanup

- [x] 2.1 `common/smol-core/src/stack.rs`: check `StackConfig::mtu`/`with_mtu` call sites; remove the dead field + builder (preferred, since the tunnel sets MTU on `ChannelDevice` directly) or wire `config.mtu` into `Stack::new`. Update call sites/docs. (stack.rs:84)
- [x] 2.2 `sdk/rust/nym-sdk-session/src/dvpn.rs`: reject a blank `id_pubkey` when building `QuicBridge` (like the empty-address / blank-`sni_host` handling), so `has_quic` never advertises an unpinned bridge. (dvpn.rs:139)
- [x] 2.3 `sdk/rust/smol-dvpn/tests/live_bringup.rs`: remove `std::process::exit(0)`; rely on the existing `tokio::time::timeout` around `tunnel.shutdown()`. Keep the `--test-threads=1` doc note. (live_bringup.rs:121)

## 3. Docs

- [x] 3.1 `sdk/rust/smol-dvpn/README.md`: reword the "no leaks / natural kill-switch you get for free" line to a scoped guarantee — only traffic through tunnel-provided sockets is protected; ordinary sockets and host DNS bypass it. (README.md:15)
- [x] 3.2 `docs/design/sdk/smol-dvpn/design.md`: clarify the SNI/cert-alt-name relationship (the "decoy" is what a DPI observer sees; the pinning verifier constrains the presented SNI to an accepted alt-name) and document the top-up headroom rationale (top-up fires at a threshold with headroom before exhaustion so the in-tunnel metadata request still fits). (design.md:267, :380)

## 4. Verify

- [x] 4.1 Add smol-core test coverage for truncated-response rejection.
- [x] 4.2 Build + test the touched crates (`smol-core`, `nym-sdk-session`, `nym-smol-dvpn`); run the live sandbox bring-up (`--test-threads=1`).
- [x] 4.3 Reply on the addressed CodeRabbit threads (pending user approval) and note the excluded findings with rationale.
