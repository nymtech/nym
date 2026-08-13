# sdk-service-provider-example

## ADDED Requirements

### Requirement: Echo service provider example
The SDK SHALL provide a runnable example `echo-service` at `sdk/rust/nym-sdk/examples/service-providers/echo-service/main.rs`, invocable via `cargo run --example echo-service`, that runs as a long-lived mixnet service provider: it connects to the mixnet with an ephemeral identity, prints its nym address on startup, and replies to every non-empty incoming message.

#### Scenario: Service starts and announces its address
- **WHEN** `cargo run --example echo-service` is executed with network access
- **THEN** the service connects to the mixnet and prints its nym address to stdout, then keeps running and listening for requests

#### Scenario: Incoming request receives an echo reply
- **WHEN** the service receives a non-empty message carrying a `sender_tag`
- **THEN** it sends back a JSON reply containing exactly the fields `message` set to `"hello"`, `timestamp_utc` set to the current server time in UTC (RFC 3339), and `request_id` set to a freshly generated random UUID v4

#### Scenario: SURB replenishment messages are ignored
- **WHEN** the service receives an empty message (SURB replenishment)
- **THEN** it skips the message without replying and continues listening

### Requirement: Replies use SURBs only
The echo service SHALL reply exclusively via `send_reply()` using the `AnonymousSenderTag` extracted from the incoming message, and SHALL NOT require, parse, or learn the requesting client's nym address.

#### Scenario: Reply without knowing the sender
- **WHEN** the service replies to a request
- **THEN** the reply is addressed by `AnonymousSenderTag` (consuming SURBs) and no client nym address appears anywhere in the service code path

### Requirement: Echo client example
The SDK SHALL provide a runnable example `echo-client` at `sdk/rust/nym-sdk/examples/service-providers/echo-client/main.rs`, invocable via `cargo run --example echo-client -- <service-nym-address>`, that sends one request to the given service address, waits for the reply, prints the parsed JSON response, and exits.

#### Scenario: Round trip against a running echo service
- **WHEN** `echo-client` is run with the nym address printed by a running `echo-service`
- **THEN** it sends a request through the mixnet, receives the JSON reply, prints the `message`, `timestamp_utc`, and `request_id` fields, and exits with status 0

#### Scenario: Missing address argument
- **WHEN** `echo-client` is run without a service address argument
- **THEN** it exits with a non-zero status and prints a usage message explaining that the service's nym address is required

### Requirement: Privacy configuration is explicit in example code
Both examples SHALL explicitly construct the client `DebugConfig` with the loop cover traffic stream and the main Poisson packet distribution enabled, and SHALL carry code comments explaining the purpose of `average_packet_delay`, `message_sending_average_delay`, `loop_cover_traffic_average_delay`, and the two disable flags.

#### Scenario: Cover traffic and timing obfuscation enabled
- **WHEN** either example builds its mixnet client
- **THEN** the passed `DebugConfig` has `cover_traffic.disable_loop_cover_traffic_stream = false` and `traffic.disable_main_poisson_packet_distribution = false`, set visibly in the example source

### Requirement: Examples compile as workspace members
The examples SHALL be declared via explicit `[[example]]` entries in `sdk/rust/nym-sdk/Cargo.toml` (cargo auto-discovery does not reach nested example directories) and SHALL compile with the workspace using existing workspace dependency pins for `uuid`, `chrono`, and `serde_json`.

#### Scenario: Examples build
- **WHEN** `cargo check --package nym-sdk --examples` is run
- **THEN** `echo-service` and `echo-client` compile without errors and without adding new crates to the workspace root

### Requirement: Service provider example README
The example directory SHALL contain `sdk/rust/nym-sdk/examples/service-providers/README.md`, written for a developer audience, that explains: what a Nym service provider is (referencing the whitepaper at https://nym.com/nym-whitepaper.pdf, §3.1 for service providers, §4.5 for SURBs, §4.6 for cover traffic); how to implement one using the SDK; how to run the example pair and what it does; and production considerations (persistent keys via `builder_with_storage.rs`, the distinction from gateway-internal service providers).

#### Scenario: A developer follows the README
- **WHEN** a developer reads the README and follows the run instructions in two terminals
- **THEN** the documented commands (`cargo run --example echo-service`, then `cargo run --example echo-client -- <printed-address>`) produce the documented JSON round trip

#### Scenario: Whitepaper is referenced
- **WHEN** the README explains what service providers are for
- **THEN** it links to https://nym.com/nym-whitepaper.pdf and cites the relevant sections

### Requirement: Top-level service-providers README
The repository SHALL contain `/service-providers/README.md` that briefly describes each gateway-internal service provider — `ip-packet-router` (tunnels IP packets over the mixnet; the exit component used by NymVPN) and `network-requester` (forwards network requests to allowed public destinations on behalf of mixnet clients) — explains that these run embedded in `nym-node`, and contains a section for developers who want to build their own service provider that links to the SDK example README by relative path.

#### Scenario: Reader is oriented and routed to the example
- **WHEN** a developer opens `/service-providers/README.md`
- **THEN** they find a description of both gateway-internal service providers and a working relative link to `sdk/rust/nym-sdk/examples/service-providers/README.md` for building their own

### Requirement: Examples run in free mode without zk-nym credentials
Both examples SHALL connect to mainnet in free mode, without zk-nym/ecash credentials: they SHALL NOT call `enable_credentials_mode()` (credentials mode is opt-in and defaults to off). The example README SHALL state that this works because the network does not currently enforce presenting a zk-nym for mixnet mode, that it lets developers run the example without holding NYM tokens, and SHALL point to the `bandwidth.rs` example for the credentials flow.

#### Scenario: Running without NYM tokens
- **WHEN** a developer with no NYM tokens and no credential setup runs the two-terminal walkthrough on mainnet
- **THEN** both examples connect and complete the JSON round trip in free mode

#### Scenario: README explains the credential posture
- **WHEN** a developer reads the example README
- **THEN** it explains why no credentials are needed today and links to `bandwidth.rs` for the paid/credentialed flow

### Requirement: Mainnet integration test gated by environment variable
The SDK SHALL provide an integration test at `sdk/rust/nym-sdk/tests/echo_example_integration.rs` that proves the example pair works against mainnet by running the built example binaries: it spawns `echo-service`, parses the nym address from its stdout, runs `echo-client` with that address, asserts a successful exit and that the printed JSON contains `message == "hello"`, an RFC 3339 `timestamp_utc`, and a valid UUID `request_id`, and always terminates the service process afterwards. The test SHALL run its body only when the `NYM_SDK_MAINNET_INTEGRATION_TESTS` environment variable is set; otherwise it SHALL print a skip message and return immediately without any network access.

#### Scenario: Skipped when the environment variable is unset
- **WHEN** `cargo test --package nym-sdk` runs without `NYM_SDK_MAINNET_INTEGRATION_TESTS` set
- **THEN** the integration test prints a skip message and returns immediately; no example binaries are spawned and no network access is attempted

#### Scenario: Expensive run proves the mainnet round trip
- **WHEN** the test is run with `NYM_SDK_MAINNET_INTEGRATION_TESTS=1 cargo test --package nym-sdk --test echo_example_integration` after `cargo build --package nym-sdk --examples`
- **THEN** it passes only if the service starts, the client round-trips through mainnet, and the reply JSON fields validate; the spawned service process is killed on both success and failure paths

#### Scenario: CI runs it only on expensive builds
- **WHEN** a pull request triggers standard CI
- **THEN** the integration test body does not run (the variable is unset); only the dedicated workflow triggered by `workflow_dispatch` or its nightly schedule sets `NYM_SDK_MAINNET_INTEGRATION_TESTS` and executes it

### Requirement: Examples follow house documentation style
Both example source files SHALL carry module-level doc comments in the existing house style: a summary, a `## What this demonstrates` section, and a fenced `sh` block with the run command, matching the pattern of `surb_reply.rs`.

#### Scenario: Doc comment style matches existing examples
- **WHEN** the example sources are reviewed alongside `surb_reply.rs`
- **THEN** each begins with `//!` module docs containing a summary, `## What this demonstrates` bullets, and the `cargo run --example` invocation
