# Gateway Probe

`nym-gateway-probe` connects to a Nym gateway node and tests whether the transports it advertises actually work: mixnet entry and exit routing, WireGuard-over-mixnet, Lewes Protocol (LP) registration, and SOCKS5 network-requester connectivity.
It also checks the WireGuard exit-policy TCP port set on its own, without the rest of the transport tests.
It is a Unix-only Rust binary and a reusable library, and it emits one structured JSON result per run: a `ProbeResult` for a standard test, or a `PortCheckResult` for a ports check.

## Why it exists

A gateway's self-description and its on-chain bond record say what the node claims to run.
Neither says whether the node answers a real connection on any of those transports.
Node-status monitoring needs a functional test, not a configuration read, before it scores a gateway as fit to route traffic.
The probe supplies that functional test: it opens a genuine Sphinx, WireGuard, LP or SOCKS5 session against the target and reports what happened, the same way a client would.

Two callers use it. An operator runs the compiled binary directly against a chosen gateway. The `nym-node-status-agent` worker links the crate as a library and drives it in process, on behalf of the [node status API](../../services/node-status-api/README.md), which schedules the audit and pays for it with ecash ticket materials it hands the agent.

## Technical details

- [Design and rationale](../../technical-details/gateway-probe/README.md): the `Probe` struct, the four run entry points, test-mode gating, and the fixed phase order.
- [Probe tests](../../technical-details/gateway-probe/probe-tests.md): what each phase tests and how, from node resolution through mixnet ping, WireGuard, LP, SOCKS5, and the ports-check flow.
- [Result shape and consumers](../../technical-details/gateway-probe/result-and-consumers.md): `ProbeResult` and `PortCheckResult`, and the signed protocol the NS agent and the node status API use to run and submit a probe.

## Code

- `nym-gateway-probe`: the crate. `src/lib.rs` (`Probe`, `do_probe_test`) drives a run. `src/main.rs` and `src/run.rs` are the CLI binary. Dated 2026-09-18, the non-Unix guard is a runtime one, not a build refusal: `src/main.rs` compiles a `#[cfg(not(unix))]` `main` that prints an error and exits non-zero, while `mod run` and the real `main` are `#[cfg(unix)]`.
- `src/common/probe_tests.rs`: the per-transport test functions, `do_ping`, `wg_probe`, `lp_registration_probe`, `do_socks5_connectivity_test`.
- `src/common/nodes.rs`: target resolution, `NymApiDirectory` for bonded gateways and `query_gateway_by_ip` for unannounced ones, and `TestedNodeDetails`.
- `src/common/wireguard.rs` and `src/common/netstack.rs`: the WireGuard tunnel tests, run through a Go netstack library. `netstack_ping/` is the Go source, and `build.rs` compiles it to a static archive and links it in.
- `src/common/socks5_test/`: the HTTPS/JSON-RPC connectivity check run over an ephemeral SOCKS5-over-mixnet client.
- `src/config/`: the CLI argument groups, `ProbeConfig`, `TestMode`, `CredentialArgs`/`CredentialMode`, `NetstackArgs`, `Socks5Args`.
- `src/common/types.rs`: the result types, `ProbeResult`, `ProbeOutcome`, `PortCheckResult`, and the per-transport result structs.

## Used by

- The [node status API](../../services/node-status-api/README.md), through `nym-node-status-agent`, which embeds this crate as a library and calls `Probe::new_for_agent` / `probe_run_agent` for a standard audit and `Probe::run_ports_for_agent` for a ports check.
- Node operators, running the binary directly (`run`, `run-local`, `run-ports` subcommands) against any gateway, bypassing the node status API entirely.

## Cross-links

- Entry gateway and exit gateway: the node roles the probe exercises. The probe is an external client of both. It shares no code with them; it only calls their public HTTP, mixnet and authenticator interfaces the same way a real client would.

## Status

Dated 2026-09-16, from the code:

- The crate's own top-level `nym-gateway-probe/README.md` describes an older CLI shape: a single `--mode` value set (`mixnet`, `single-hop`, `two-hop`, `lp-only`), a bare `run-local` subcommand, and a `--only-wireguard` flag. The code in `src/run.rs` and `src/config/` has moved on: four subcommands (`run`, `run-local`, `run-ports`, `run-agent`), a `TestMode` with six values (`core`, `wg-mix`, `wg-lp`, `lp-only`, `socks5-only`, `all`), and no `--only-wireguard` flag. This specification and the source are authoritative; the crate README needs a refresh.
