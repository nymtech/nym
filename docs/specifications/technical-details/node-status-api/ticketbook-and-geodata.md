# Ticketbook issuance and geodata

This covers two subsystems that both reach outside the service's own database: the ecash ticketbook manager, which keeps a private buffer of tickets to pay for test runs, and the current geodata lookup that the monitor uses to place nodes on a map. This document groups them because both depend on an external system: the ecash signing quorum and chain for ticketbooks, and `ipinfo.io` for geodata. The ticketbook spine is `openspec/specs/node-status-api-ticketbook/spec.md`.

## Why the service needs its own ticketbooks

Every gateway test run spends an ecash ticket against the gateway under test, the same as any client connection. The service cannot depend on a human topping up its credential balance on the same schedule it runs tests, so it runs a ticketbook manager that deposits on `nyxd` and collects threshold-signed ticketbooks from the ecash signer quorum, on its own timed loop, independent of the monitor and the test-run queuer.

### Relationship to `nym-credential-proxy`

The ticketbook manager is not a client of the deployed `nym-credential-proxy` HTTP service. It embeds `common/credential-proxy` (crate `nym-credential-proxy-lib`) directly: `nym_credential_proxy_lib::deposits_buffer` for the deposit and chunking logic, `nym_credential_proxy_lib::shared_state::ecash_state` for the ticket types and epoch state, and `nym_credential_proxy_lib::quorum_checker::QuorumStateChecker` for tracking whether the signing quorum is available. `main.rs` constructs a `ChainClient` from the same crate and passes it straight into the ticketbook manager's state. The node status API and `nym-credential-proxy` are two independent processes that share code, not two processes where one calls the other.

## The buffer loop

`TicketbookManager::run` (`ticketbook_manager/mod.rs`) ticks on an interval (default 60s, `NYM_NODE_STATUS_API_TICKETS_CHECK_INTERVAL`), prioritising shutdown in a `biased` `tokio::select!`. Each tick it confirms credentials are issuable, checks that the ecash quorum is available, and for each buffered ticket type refills that type's buffer independently:

1. If available tickets are already at or above the configured buffer size (default 50, `NYM_NODE_STATUS_API_TICKETS_BUFFER`), do nothing.
2. Otherwise compute the number of ticketbooks needed by ceiling division of the ticket shortfall over tickets-per-ticketbook.
3. Deposit on chain in chunks bounded by `max_concurrent_deposits` (default 5, `NYM_NODE_STATUS_API_MAX_CONCURRENT_DEPOSITS`), checking for shutdown between chunks, never mid-chunk, so an in-flight chain operation is never abandoned.
4. Within a chunk, make every deposit first, then obtain the corresponding ticketbook for each, one deposit at a time.

The default buffered ticket types are `V1MixnetEntry`, `V1WireguardEntry`, `V1WireguardExit`, and a second `V1WireguardEntry` for the Lewes protocol tests, each refilled to its own buffer size independently.

## Aggregation failure

When the manager fails to obtain an aggregated wallet from the quorum for a deposit, it inserts a pending-ticketbook row (the serialized issuance data, the deposit id, the expiration date, the epoch, and the failure message) and stops the refill for that ticket type. No code path reads the pending-ticketbook table or retries the aggregation. The manager requests no ticketbook for the remaining deposits in the same chunk. On the next tick the buffer is still short, and the manager deposits again.

## Startup cache and material assignment

At startup, `TicketbookManagerState::build_initial_cache` warms the epoch, deposit amount, master verification keys, threshold, and signatures, so the buffer loop and material assignment do not re-fetch them every cycle. `has_enough_ticketbooks` checks the buffered count per type before an assignment is allowed to proceed. `attempt_assign_ticket_materials(testrun_id)` pulls one next-ticket per buffered type, atomically incrementing the spent count and linking the ticket to the test run in the same statement (see [Persistence](persistence.md#concurrent-claims)), and collects the per-epoch verification key plus coin-index and expiration-date signatures into the `AttachedTicketMaterials` that [Test-runs](testruns.md#the-wire-contract) sends to the agent. The retrieved usable index is `spent - 1`, and reading it errors if `spent` is `0`.

## Geodata

The monitor's node-to-location lookup lives in `monitor/geodata.rs` and is described in full in [Monitor cycle](monitor-cycle.md#geodata-is-cached-only-on-success): a `moka` cache keyed by node id, TTL `geodata_ttl` (default 86400s), populated by trying each of a node's declared IPs against `ipinfo.io` in order and caching only the first success. A node that cannot be geolocated is never cached as a failure, so it is retried, and its `ipinfo` quota spent, on every cycle.

This lookup is local to the node status API. The service does not read the on-chain geolocation contract (`contracts/geolocation`), and nothing here verifies who asserted a location or when.

## Cross-links

- [Test-runs](testruns.md#the-wire-contract): where `AttachedTicketMaterials` are consumed.
- [Persistence](persistence.md#concurrent-claims): the atomic ticket-claim statement.

## Technical notes

- **Implementation**: `ticketbook_manager/mod.rs` (`TicketbookManager::run`, `check_ticketbooks_buffer`, `maybe_refill_ticketbook`), `ticketbook_manager/state.rs` (`TicketbookManagerState`, `build_initial_cache`, `has_enough_ticketbooks`, `attempt_assign_ticket_materials`), `ticketbook_manager/storage/auxiliary_models.rs` (`RetrievedTicketbook`, `StoredIssuedTicketbook`, zeroized on drop), `db/queries/ecash_data.rs`, `monitor/geodata.rs` (`IpInfoClient`, `Location`, `ExplorerPrettyBond`).
- **Config**: `--tickets-buffer-size` (default 50), `--max-concurrent-deposits` (default 5), `--tickets-buffer-check-interval` (default 1m), `--quorum-check-interval` (default 5m), `--buffered-ticket-types`, `--mnemonic`, `--ecash-client-identifier-bs58`.
- **External services**: `nyxd` (deposits), the ecash signing quorum (threshold signing) via the embedded `nym-credential-proxy-lib`, and `ipinfo.io` (geolocation).
