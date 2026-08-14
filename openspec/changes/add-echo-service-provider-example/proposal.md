# Add Echo Service Provider Example

## Why

The Nym SDK has no example showing how to build a service provider — a long-running service reachable anonymously over the mixnet — even though this is the core integration pattern the whitepaper describes for third-party applications (§3.1). Developers currently have to reverse-engineer the pattern from `surb_reply.rs` (which only sends to self) or from the gateway-embedded service providers, which are a different, heavier pattern. There is also no top-level documentation in `/service-providers` explaining what lives there or how it relates to SDK-level service providers.

## What Changes

- Add a new `sdk/rust/nym-sdk/examples/service-providers/` directory containing two cargo examples:
  - `echo-service`: a long-running service provider built on the SDK stream module (`client.listener()` / `MixnetStream`, the socket-like `AsyncRead + AsyncWrite` API) that replies to every request via SURBs with a JSON payload containing `"hello"`, the server time in UTC, and a random UUID v4 request id.
  - `echo-client`: a companion example that takes the service's nym address as a CLI argument, opens a stream, sends one request, prints the JSON reply, and exits.
- Both examples explicitly construct the client `DebugConfig` with cover traffic and Poisson timing obfuscation enabled (the defaults), with comments teaching what each knob does.
- Both examples run in free mode without zk-nym credentials (the network does not currently enforce them for mixnet mode), so anyone can run them on mainnet without NYM tokens.
- Add explicit `[[example]]` entries to `nym-sdk/Cargo.toml` (cargo auto-discovery does not reach nested example directories) and wire `uuid`, `time`, and `serde_json` as dev-dependencies from existing workspace pins.
- Add `sdk/rust/nym-sdk/examples/service-providers/README.md` explaining, for a developer audience: what a Nym service provider is (with whitepaper references), how to implement one, and how to run the example.
- Add `/service-providers/README.md` listing the gateway-internal service providers (`ip-packet-router`, `network-requester`) with brief descriptions, plus a "build your own" section linking to the new example README.
- Add an end-to-end integration test that runs the built `echo-service` and `echo-client` example binaries against mainnet and asserts the JSON round trip. The test is gated by the `NYM_SDK_MAINNET_INTEGRATION_TESTS` environment variable — unset (the ordinary `cargo test` case) it skips immediately; it only runs on expensive CI runs via a dedicated scheduled/manually-dispatched workflow that sets the variable.

- Fix a message race in the SDK stream module discovered by the integration test: `Data` frames that overtake their stream's `Open` through the mixnet (which routes every message independently) were silently dropped before the stream was registered, hanging both peers. The router now buffers such orphan frames (bounded per stream, across streams, and by TTL) and drains them into the stream's reorder buffer on registration.

## Capabilities

### New Capabilities

- `sdk-service-provider-example`: Runnable SDK examples and documentation demonstrating the service-provider pattern — anonymous request/reply over the mixnet using SURBs, with cover traffic and timing obfuscation explicitly configured.

### Modified Capabilities

<!-- none — this change adds examples and documentation only; no existing spec-level behavior changes -->

## Impact

- `sdk/rust/nym-sdk/Cargo.toml`: new `[[example]]` entries and dev-dependencies (`uuid`, `time`, `serde_json` — all already pinned in the workspace root).
- `sdk/rust/nym-sdk/examples/service-providers/`: new directory (two example sources + README).
- `/service-providers/README.md`: new file; no code in `ip-packet-router` or `network-requester` is touched.
- `sdk/rust/nym-sdk/tests/`: new env-var-gated integration test; `.github/workflows/`: new workflow for expensive runs (schedule + `workflow_dispatch`), following the existing `nym-api-integration-tests.yml` pattern.
- `sdk/rust/nym-sdk/src/mixnet/stream/mod.rs`: orphan-frame buffering in the stream router (the one library change, a bugfix with unit tests; no public API changes). Examples compile as part of the workspace (`cargo build --examples`); default PR CI cost is unchanged (the integration test is skipped unless explicitly requested).
