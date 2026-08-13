# Design: Add Echo Service Provider Example

## Context

The Nym SDK (`sdk/rust/nym-sdk`) ships ~25 flat examples plus subdirectory examples (`libp2p_chat/`, etc.). `surb_reply.rs` already demonstrates the SURB reply primitive (`AnonymousSenderTag` + `send_reply()`) but only by sending to self. The whitepaper (§3.1, §4.5, §4.6) positions "service providers" — third-party services reachable anonymously via the mixnet — as the primary integration pattern, yet no example shows one.

Two distinct things in this repo are called "service provider":

1. **Gateway-internal service providers** (`/service-providers`): `ip-packet-router` and `network-requester`, embedded in `nym-node`, built on `nym-service-providers-common`.
2. **SDK-level service providers**: any process with a mixnet client that listens for requests and replies via SURBs. This is what the example demonstrates.

Relevant facts established during exploration:

- Cover traffic and Poisson timing obfuscation are **on by default** in `nym-client-core` (`common/client-core/config-types/src/lib.rs`: `disable_loop_cover_traffic_stream: false`, `disable_main_poisson_packet_distribution: false`).
- `MixnetClientBuilder::debug_config()` accepts a `DebugConfig` (re-exported as `nym_sdk::DebugConfig`).
- Cargo auto-discovers `examples/*.rs` and `examples/*/main.rs` only — one directory level. Anything deeper needs explicit `[[example]]` entries.
- Workspace root already pins `uuid = "1.19.0"`, `chrono = "0.4.41"`, `serde_json`.
- `service-providers/network-requester/README.md` is license boilerplate only; `ip-packet-router` has no README; `/service-providers` has no top-level README.

All user-facing design decisions were settled interactively before this proposal (see Decisions).

## Goals / Non-Goals

**Goals:**

- A runnable, self-contained demonstration of the service-provider pattern: anonymous request → JSON reply via SURBs.
- Teach the privacy configuration surface: the example explicitly constructs `DebugConfig` with cover traffic and timing obfuscation enabled, with explanatory comments.
- Developer-audience documentation: what a service provider is (grounded in the whitepaper), how to implement one, how to run the example.
- Orient readers of `/service-providers` and clearly separate gateway-internal SPs from SDK-level SPs.

**Non-Goals:**

- No changes to SDK library code, `nym-service-providers-common`, or the gateway-internal service providers.
- No persistent key storage in the example (ephemeral keys; production key persistence is a README note pointing at `builder_with_storage.rs`).
- No service credentials / paid-access flow (whitepaper §3.2 steps 5–6) — mentioned in the README as context only.
- Default PR CI stays compile-only (`cargo build --examples` / `cargo check`); the live-network integration test runs only on expensive, explicitly triggered or scheduled CI runs.

## Decisions

### D1: Two examples, both run via `cargo run --example`

`echo-service` (long-running server) is paired with `echo-client` (one-shot requester). A server alone cannot be demonstrated; the pair enables a concrete two-terminal walkthrough in the README. No `[[bin]]` targets — these are examples, not shipped binaries. *(User decision.)*

### D2: Explicit `[[example]]` entries in `nym-sdk/Cargo.toml`

`examples/service-providers/echo-service/main.rs` and `examples/service-providers/echo-client/main.rs` are two levels deep, beyond cargo auto-discovery. Explicit entries also let the `service-providers/` example directory host future SP examples. Alternative considered: flat files `examples/echo_service.rs` — rejected because the user specified the directory and it groups the README with the code.

### D3: JSON response payload via `serde_json`

```json
{ "message": "hello", "timestamp_utc": "<RFC 3339>", "request_id": "<UUID v4>" }
```

Models a realistic service response and gives the client something to parse. Serialization via a `#[derive(Serialize, Deserialize)]` struct shared by constructing the same struct definition in each example (examples cannot share modules without extra plumbing; duplicating a 3-field struct is simpler than a shared helper crate). Alternative considered: plain text — rejected as less realistic. *(User decision: JSON.)*

### D4: Ephemeral keys for both examples

Both examples use `MixnetClientBuilder::new_ephemeral()` (or TempDir-backed storage matching `surb_reply.rs` house style). Consequence: the service's nym address changes each run, so the run workflow is *start service → it prints its address → pass address as CLI arg to client*. The "persist your keys — your address is your identity" production consideration moves to the README, referencing `builder_with_storage.rs`. *(User decision: ephemeral.)*

### D5: Explicit `DebugConfig` construction showing privacy knobs

Although cover traffic and Poisson timing are defaults, both examples explicitly build a `DebugConfig`, visibly set / assert the relevant fields, and comment what each does:

- `traffic.average_packet_delay` — per-hop mixing delay
- `traffic.message_sending_average_delay` — Poisson sending rate (timing obfuscation)
- `traffic.disable_main_poisson_packet_distribution = false` — keep Poisson stream on
- `cover_traffic.loop_cover_traffic_average_delay` — loop cover rate
- `cover_traffic.disable_loop_cover_traffic_stream = false` — keep loop cover on

Rationale: the example must *teach* the config surface, not silently inherit it. The README also names `set_no_cover_traffic()` / `set_no_poisson_process()` as debug-only escape hatches and warns against them in production. Alternative considered: rely on defaults with a comment — rejected as invisible to readers skimming code.

### D6: Reply exclusively via SURBs

The service never learns and never asks for the client's address: it extracts `sender_tag` from `ReconstructedMessage` and uses `send_reply()`. Empty messages (SURB replenishment) are skipped, matching `surb_reply.rs`. This is the core teaching point (whitepaper §4.5).

### D7: Documentation structure

- Module doc comments in house style (`//! ## What this demonstrates`, run instructions in a ```sh fence) on both examples.
- `examples/service-providers/README.md`: what an SP is (whitepaper §3.1 quote + link to https://nym.com/nym-whitepaper.pdf), how SURBs (§4.5) and cover traffic/unobservability (§4.6, §3.5) apply, how to implement one (annotated walkthrough), how to run the pair, production notes (key persistence, the gateway-internal distinction).
- `/service-providers/README.md`: one-paragraph descriptions of `ip-packet-router` (tunnels IP packets; exit layer for NymVPN) and `network-requester` (SOCKS5/HTTP proxy for allowed destinations), the gateway-internal vs SDK-level distinction, and a "build your own" section linking to the example README by relative path.

### D8: End-to-end integration test, gated to expensive CI runs

A single integration test at `sdk/rust/nym-sdk/tests/echo_example_integration.rs`, gated by an environment variable: unless `NYM_SDK_MAINNET_INTEGRATION_TESTS` is set, the test prints a skip message and returns immediately, so `cargo test` on PRs touches no network and costs nothing. Alternatives considered: `#[ignore]` (the crate's existing pattern in `tcp_proxy_server.rs`) — rejected per user decision in favor of a single explicit env-var gate; a cargo feature flag — non-standard in this repo. Trade-off: a skipped run reports `ok` rather than `ignored`, mitigated by the printed skip message. It runs the *built example binaries* rather than reimplementing the logic in-process:

1. Spawn `target/<profile>/examples/echo-service` (built beforehand by the workflow via `cargo build --package nym-sdk --examples`), capture stdout.
2. Parse the printed nym address (with a startup timeout generous enough for mainnet gateway registration).
3. Spawn `target/<profile>/examples/echo-client -- <address>`, assert exit status 0.
4. Parse the client's printed JSON and assert: `message == "hello"`, `timestamp_utc` parses as RFC 3339, `request_id` parses as a UUID.
5. Kill the service process on completion or timeout (no orphaned processes on CI runners).

Rationale for spawning the real examples: the examples *are* the deliverable; running them proves the documented two-terminal walkthrough works against mainnet, whereas an in-process test would duplicate the code and could pass while the examples rot. `CARGO_BIN_EXE_*` env vars only exist for `[[bin]]` targets, so the test locates example binaries relative to the test executable's path (standard `current_exe()` → `target/<profile>/examples/` approach).

CI gating: a new dedicated workflow (`ci-sdk-example-integration-tests.yml`) modeled on `nym-api-integration-tests.yml`, triggered by `workflow_dispatch` and a nightly `schedule` — not by PRs (mainnet-dependent tests are too flaky and slow for the PR gate). It builds the examples, then runs `cargo test --package nym-sdk --test echo_example_integration -- --nocapture` with `NYM_SDK_MAINNET_INTEGRATION_TESTS: "1"` in the job env.

### D9: Free mode — no zk-nym credentials

Both examples (and therefore the integration test) run in free mode, without zk-nym/ecash credentials. The mixnet does not currently enforce presenting a zk-nym for mixnet mode, so this keeps the example simple and lets anyone run it on mainnet without holding NYM tokens. Mechanically this means *not* calling `MixnetClientBuilder::enable_credentials_mode()` — credentials mode is opt-in in the SDK (`enabled_credentials_mode` defaults to `false`, see `sdk/rust/nym-sdk/src/mixnet/client.rs:280`). The README states this explicitly and points to the existing `bandwidth.rs` example for the credentials/ticketbook flow. *(User decision.)*

## Risks / Trade-offs

- [Examples hit the live mainnet and need a working gateway connection] → This matches every existing SDK example; README states network access is required and startup may take a few seconds.
- [Ephemeral address changes each run could confuse users scripting against it] → README makes the print-address-then-pass-to-client workflow explicit and points to `builder_with_storage.rs` for stable addresses.
- [Duplicated response-struct definition between the two examples could drift] → Struct is 3 fields; README and doc comments note the client parses exactly what the service sends. Acceptable for example code.
- [`DebugConfig` fields are semver-exempt debug surface and may change] → Example compiles in-workspace, so any field rename breaks the build immediately and gets fixed with the SDK change.
- [Whitepaper URL could move] → Also cite section numbers so the reference remains resolvable via search.
- [Mainnet flakiness makes the integration test non-deterministic] → Test runs only on scheduled/dispatched expensive runs, never gates PRs; generous connect/reply timeouts; the service process is always killed on exit paths.
- [Network may later enforce zk-nym credentials for mixnet mode, breaking free mode] → The README notes free mode reflects current network policy; the nightly integration test will surface enforcement changes immediately, and the fix is switching the examples to the documented `bandwidth.rs` credentials flow.

## Open Questions

None — all decisions were settled with the user during exploration.
