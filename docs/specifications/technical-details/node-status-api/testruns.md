# Test-runs

This covers gateway test-run orchestration: the background queuer that enqueues probe and ports-check work for bonded gateways, and the authenticated `/internal/testruns*` API that assigns work to external agents and ingests their signed results. The spine here is `openspec/specs/node-status-api-testruns/spec.md` and `openspec/specs/node-status-agent/spec.md` (both revised or generated 2026-08-03 / 2026-07-07), verified against `src/testruns`, `src/http/api/testruns.rs`, `src/http/state.rs` and `nym-node-status-agent`.

## Enqueuing work

The queuer loops: re-queue stale in-progress runs (assigned before the stale window, default 7200s, `NODE_STATUS_API_TESTRUN_STALE_IN_PROGRESS`), then walk all gateways ordered by last-testrun time, filter to bonded gateways, and queue a probe run per gateway, then sleep the refresh interval (default 450s, `NODE_STATUS_API_TESTRUN_REFRESH_INTERVAL`). Enqueue is idempotent: a non-complete Probe run for a gateway is reused rather than duplicated.

A separate ports-check scheduler runs every 10 minutes and enqueues `Queued`/`PortsCheck` rows for bonded gateways whose last ports check is null or older than 14 days, deduplicating against any pending PortsCheck run. It is enabled when `PORTS_CHECK_SCHEDULER_ENABLED` is unset or set to one of `1`, `true`, `TRUE`, `yes`, `YES`, and disabled for any other value, so a typo disables it silently. Its first tick fires one period after startup, and a missed tick is skipped rather than made up.

## Assignment: authentication, freshness, capacity, and a process-local mutex

`GET /internal/testruns` and `GET /internal/testruns/ports-check` each authenticate the request (agent public key in the configured `agent_key_list`, valid ed25519 signature), enforce request freshness (default 120s, `AGENT_REQUEST_FRESHNESS`), reject when the in-progress count is at or above `agent_max_count` (default 40, `NYM_NODE_STATUS_API_MAX_AGENT_COUNT`, 503), take an assignment mutex, require enough buffered ticketbooks of every type, then atomically claim the oldest queued run of the matching kind for a bonded gateway with performance greater than zero (`FOR UPDATE ... SKIP LOCKED`), attach ticket materials, and return a `TestrunAssignmentWithTickets`.

The capacity check differs by route: the probe route counts all in-progress runs regardless of kind, the ports-check route counts only in-progress `PortsCheck` runs. Failure statuses: 401 for an unregistered key, a bad signature, or a stale timestamp; 503 with the body `No testruns available` when at capacity or nothing is queued; 500 with `Internal server error` when the ticketbook store cannot be checked, holds too few ticketbooks, or ticket materials cannot be assigned. A ticketbook shortage is deliberately not a 503, so agents treat it as a hard error rather than an idle poll.

### The mutex is process-local

The assignment mutex serializes the count check, the ticketbook-sufficiency check and the claim within one API process only. The atomic `SKIP LOCKED` claim still stops two agents ever receiving the same run, but with more than one API replica against the same database, both `agent_max_count` and the ticketbook-sufficiency check can be exceeded, because each replica evaluates them under its own lock. Single-instance deployment is an implicit requirement of the current design; scaling the API horizontally needs both checks moved into the database or another shared lock.

### Claim and material allocation are not one transaction

The claim flips the run to `InProgress` and commits. Ticket allocation then runs as a separate sequence in which each ticket's `used_tickets` counter is incremented by its own atomic statement before the ticket is linked to the run. A failure anywhere in that sequence leaves the run `InProgress` with no materials returned (the request answers 500), while every ticket already incremented stays spent, unlinked to any run if the failure landed between increment and link. Neither the run nor the tickets roll back: the run stays unavailable until the stale sweep re-queues it, and the spent tickets never return to the buffer, so repeated failures drain it and force fresh deposits.

## The wire contract

Both request endpoints are `GET` with a JSON body: `{ "payload": { "agent_public_key": [<32 numbers>], "timestamp": <unix seconds> }, "signature": [<64 numbers>] }`. On this surface, ed25519 keys and signatures are byte arrays, not base58 strings. The signature is verified as ed25519 over `bincode::serialize(payload)`, so the payload's field order and types are part of the signing contract.

A successful assignment flattens the assignment fields at the top level next to `ticket_materials`:

```json
{
  "testrun_id": 1,
  "assigned_at_utc": 1751894748,
  "gateway_identity_key": "<base58 ed25519>",
  "last_ports_check_utc": null,
  "ticket_materials": {
    "coin_indices_signatures":    [ { "data": [/* bytes */], "revision": 1 } ],
    "expiration_date_signatures": [ { "data": [/* bytes */], "revision": 1 } ],
    "master_verification_keys":  [ { "data": [/* bytes */], "revision": 1 } ],
    "attached_tickets":           [ { "ticketbook": { "data": [/* bytes */], "revision": 1 }, "usable_index": 0 } ]
  }
}
```

All ticket blobs are byte arrays, which makes these responses hundreds of kilobytes. All three submission endpoints answer 201 with an empty body on success. The `/internal/testruns` router accepts bodies up to 5 MiB (413 beyond that). The v1 submission endpoint converts every JSON extractor rejection into 400 with the rejection text as the body; the two v2 endpoints surface axum's own rejections (415, 400, 422 as appropriate).

## Submission handling differs between v1 and v2

`POST /internal/testruns/{id}` (v1) requires the run to be in-progress and its stored `assigned_at_utc` to match the submission, resolves the gateway, and marks the run Complete. It does not write any score: the helper still named `update_gateway_score` only stamps `last_testrun_utc` and `last_updated_utc` (see [Persistence](persistence.md)). Its rejection bodies are specific: "not found in progress state," a timestamp-mismatch message, or "Invalid probe_result."

`POST /internal/testruns/{id}/v2` looks the run up by id in any status, rejects a gateway-identity mismatch with 400, rejects a timestamp mismatch with 400, and for an unknown id creates the gateway if needed and inserts an external `InProgress`/`Probe` run before processing. On success, probe v2 completes every in-progress run for that gateway, not only the submitted id.

Submission writes are not atomic: every path issues separate autocommit statements on one pooled connection. The probe paths flip status to `Complete` first and only then write the probe log, result and timestamps, so a crash after the status write leaves a completed run whose result was never stored, and nothing ever re-queues a `Complete` run. The ports-check path writes the summary onto the gateway before marking the run complete, the opposite order, so an interruption there leaves the run re-runnable instead.

`POST /internal/testruns/{id}/ports-check/v2` validates kind, gateway identity and timestamp in that order for a known id, rejecting each with its own 400 message. For an unknown id it creates the gateway and inserts an external run. It writes only the compact `{ all_pass, error, port_check_target, failed_ports }` summary plus a server-clock `last_ports_check_utc`, marks only the submitted run Complete, and never persists a probe log.

## Stale re-queue can discard an in-flight result

A run in progress longer than the stale window gets set back to `Queued` and can be claimed by a second agent, overwriting `last_assigned_utc`. If the original agent finishes and submits afterward, its `assigned_at_utc` no longer matches, so the v1 endpoint rejects it with 400 and the probe result is dropped, even though the probe itself succeeded. Lengthening probe duration, or shortening the stale window, makes this happen more often.

## The agent side

`nym-node-status-agent` is a one-shot CLI, looped externally by its container `entrypoint.sh`. It exposes `run-probe`, `run-ports-check` and `generate-keypair` subcommands, and requires `NODE_STATUS_AGENT_AUTH_KEY` (a base58 ed25519 private key) for the first two. Given several `--server` values, it treats the first as primary and requests an assignment only from it, but fans result submissions out to every configured server concurrently: the primary gets a v1 submission with a probe log truncated to 1024 bytes, and each secondary gets a v2 submission carrying the gateway identity key with an untruncated log. A per-server submission failure is logged as a warning and does not block the others.

For `run-probe`, the agent requests a run, and on "no work available" logs and exits successfully without probing. On an assignment it builds the probe via `Probe::new_for_agent`, converts the attached ticket materials into credential arguments, runs the probe in process (not as a subprocess, see `openspec/specs/architecture/spec.md`), captures tracing output only for the duration of the run, and submits.

For `run-ports-check`, the agent treats both "no testruns available" and HTTP 404 as no work, forces the netstack port list to the full exit-policy port set, and runs the scan under a 90-minute hard timeout; a timeout logs an error and returns without submitting. On success it submits to every configured server, including the primary, via the v2 ports-check endpoint.

Every request is ed25519-signed over the bincode serialization of its payload; there is no bearer-token or TLS-client-certificate authentication.

## Cross-links

- [Ticketbook issuance and geodata](ticketbook-and-geodata.md): where the ticket materials attached to an assignment come from.
- [Persistence](persistence.md): the `testruns` table and the atomic claim statement.
- [HTTP surface](http-surface.md): how a completed test run's probe result reaches `/v2/gateways` and the dVPN directory.

## Technical notes

- **Implementation**: `testruns/mod.rs` (`start`, `refresh_stale_testruns`), `testruns/queue.rs` (`try_queue_testrun`), `http/api/testruns.rs` (request and submit handlers, 5 MiB body limit), `http/state.rs` (`authenticate_agent_submission`, `is_fresh`, `testrun_assignment_guard`), `http/server.rs` (ports-check scheduler), `db/queries/testruns.rs` (atomic claim, external inserts, enqueue-due). Agent: `nym-node-status-agent/src/cli/{run_probe,run_ports_check,generate_keypair}.rs`, `common.rs`, `log_capture.rs`. Client: `nym-node-status-client/src/{lib,auth,models}.rs`.
- **Models**: `TestRunStatus { Queued, InProgress, Complete }`, `TestRunKind { Probe, PortsCheck }`.
