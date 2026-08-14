# Tasks: Add Echo Service Provider Example

## 1. Cargo wiring

- [x] 1.1 Add `uuid` (with `v4` feature), `time` (with `formatting`/`parsing` features; chrono was swapped out per review — unmaintained), and `serde_json` to `nym-sdk` dev-dependencies using workspace pins; verify `serde` derive availability for the examples
- [x] 1.2 Add `[[example]]` entries for `echo-service` and `echo-client` pointing at `examples/service-providers/echo-service/main.rs` and `examples/service-providers/echo-client/main.rs`

## 2. Echo service example

- [x] 2.1 Implement `examples/service-providers/echo-service/main.rs`: ephemeral client in free mode (no `enable_credentials_mode()`), explicit `DebugConfig` with cover traffic + Poisson timing enabled and commented knobs, print nym address on startup
- [x] 2.2 Implement the request loop: skip empty (SURB replenishment) messages, extract `AnonymousSenderTag`, build the JSON response (`message: "hello"`, `timestamp_utc` RFC 3339, `request_id` UUID v4), reply via `send_reply()`
- [x] 2.3 Add module doc comment in house style (`## What this demonstrates` + run command)

## 3. Echo client example

- [x] 3.1 Implement `examples/service-providers/echo-client/main.rs`: parse service nym address from CLI arg (usage message + non-zero exit when missing), ephemeral client in free mode with the same explicit `DebugConfig`
- [x] 3.2 Send one request, wait for the reply (skipping empty messages), parse and print the JSON fields, exit 0
- [x] 3.3 Add module doc comment in house style

## 4. Example README

- [x] 4.1 Write `examples/service-providers/README.md`: what a service provider is (whitepaper link + §3.1, §4.5 SURBs, §4.6 cover traffic), how to implement one (annotated walkthrough of the example), two-terminal run instructions with expected output
- [x] 4.2 Add production notes: persistent keys (`builder_with_storage.rs`), debug-only escape hatches (`set_no_cover_traffic()` / `set_no_poisson_process()`) and why not to use them, distinction from gateway-internal service providers
- [x] 4.3 Add credential-posture section: examples run in free mode without zk-nym credentials (network does not currently enforce them for mixnet mode; no NYM tokens needed), pointing to `bandwidth.rs` for the credentials flow

## 5. Top-level service-providers README

- [x] 5.1 Write `/service-providers/README.md`: describe `ip-packet-router` and `network-requester`, note they run embedded in `nym-node`, add "build your own" section with relative link to the example README

## 6. Mainnet integration test (expensive CI only)

- [x] 6.1 Implement `sdk/rust/nym-sdk/tests/echo_example_integration.rs` gated on the `NYM_SDK_MAINNET_INTEGRATION_TESTS` env var (print skip message and return immediately when unset): locate built example binaries via the test executable's path (`target/<profile>/examples/`), spawn `echo-service`, parse its printed nym address from stdout with a generous startup timeout
- [x] 6.2 Run `echo-client` against the parsed address; assert exit 0 and that the printed JSON validates (`message == "hello"`, RFC 3339 `timestamp_utc`, UUID `request_id`); ensure the service process is killed on all exit paths (success, failure, timeout)
- [x] 6.3 Add `.github/workflows/ci-sdk-example-integration-tests.yml` modeled on `nym-api-integration-tests.yml`: triggers `workflow_dispatch` + nightly `schedule`, builds examples (`cargo build --package nym-sdk --examples`), runs `cargo test --package nym-sdk --test echo_example_integration -- --nocapture` with `NYM_SDK_MAINNET_INTEGRATION_TESTS: "1"` in the job env
- [x] 6.4 Confirm the test skips (no example spawn, no network access) under plain `cargo test --package nym-sdk` with the env var unset

## 7. Stream router race fix (review-driven scope extension)

- [x] 7.1 Reproduce and root-cause the intermittent mainnet failure: `Data` overtaking `Open` is silently dropped for unregistered stream ids in `send_to_stream`
- [x] 7.2 TDD: failing unit test for data-before-registration, then implement the bounded orphan buffer in `StreamMap` (drain on registration, caps per stream/across streams, TTL sweep in `cleanup_stale`)
- [x] 7.3 Unit tests for sequencing, TTL sweep, and both capacity bounds; full `nym-sdk --lib` suite green

## 8. Verification

- [x] 8.1 `cargo check --package nym-sdk --examples` passes with no new workspace crates
- [x] 8.2 Run the two-terminal walkthrough against the live network without any credential setup or NYM tokens: service prints address, client round-trips and prints the JSON reply exactly as documented in the READMEs
- [x] 8.3 Run the integration test locally with `NYM_SDK_MAINNET_INTEGRATION_TESTS=1` and confirm it passes against mainnet (three consecutive passes post-race-fix)
- [x] 8.4 Verify README links resolve (relative link from `/service-providers/README.md`, whitepaper URL) and doc-comment style matches `surb_reply.rs`
