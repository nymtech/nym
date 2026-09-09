# smolmix-connection-probe Spec Delta

## ADDED Requirements

### Requirement: Repeated establishment against random exits

The probe SHALL run tunnel establishment on the auto-discovery path a configurable number of times (default at least 100), each with a fresh random client id and no pinned IPR, so each run registers a fresh entry gateway and auto-discovers a fresh performance-weighted random exit. Each run SHALL use a fresh WASM instance, because the tunnel singleton allows only one establishment per instance.

#### Scenario: Each run targets a fresh random exit

- **WHEN** the probe runs the configured number of establishments
- **THEN** each run uses a fresh random client id and no pinned IPR, so the exit differs run to run rather than being reused

#### Scenario: A fresh instance per run

- **WHEN** the probe starts a run
- **THEN** it uses a WASM instance that has not previously established a tunnel, so the one-shot establishment guard does not block it

### Requirement: Connection outcome is classified per run

For each run the probe SHALL record, from the tunnel's own log output: the entry gateway it registered with, the number of exits attempted, whether any attempt fell back from the newer protocol version to the older one, whether it rotated to a different exit after a failure, and the final outcome. On success it SHALL record the winning exit address, the negotiated version and MTU, and the elapsed time. On failure it SHALL record a reason distinguishing no eligible exits, a gateway-registration failure, exhaustion of all attempts, and other errors. Recording the entry gateway on every run lets a run that failed against many exits be told apart from an entry gateway that is itself at fault.

#### Scenario: A rotating, downgrading connect is recorded

- **WHEN** a run tries one exit that times out on the newer version, falls back to the older version, fails, and connects on a second exit
- **THEN** the run records attempts = 2, downgrade = yes, rotated = yes, outcome = connected, with the second exit's address, version, and elapsed time

#### Scenario: A total failure is recorded with a reason

- **WHEN** a run exhausts its attempts without connecting
- **THEN** the run records outcome = failed with a reason (no eligible exits, gateway registration failed, attempts exhausted, or other)

#### Scenario: The entry gateway is recorded on every run

- **WHEN** any run completes, whether it connected or failed
- **THEN** its row names the entry gateway it registered with, so failures concentrated on one gateway are visible across the results

### Requirement: Per-resolver DoH probe

On a successful connection the probe SHALL resolve a fixed hostname against each default DoH resolver individually (`1.1.1.1`, `8.8.8.8`, `9.9.9.9`), by setting the endpoint list to that one resolver, and SHALL classify each as resolved, timed out, rate-limited (HTTP 429), or a server error. It SHALL also run one resolution against the full default endpoint list and record whether it rotated to a backup endpoint.

#### Scenario: A rate-limited resolver is recorded as 429

- **WHEN** a resolver answers HTTP 429 to the probe's query
- **THEN** that resolver's verdict for the run is rate-limited, not a timeout or a generic failure

#### Scenario: A single-endpoint timeout is attributed to that resolver

- **WHEN** the probe queries one resolver and it does not answer within the budget
- **THEN** the timeout is attributed to that specific resolver, because only it was in the endpoint list

### Requirement: Results written to a markdown file

The probe SHALL write its results to a markdown file (`results.md`) containing a table of per-run connection outcomes, a table of per-run DoH verdicts, and a summary of aggregate rates (success rate, downgrade rate, mean and maximum attempts, and per-resolver rate-limit and timeout rates). The file SHALL be a generated artifact, not committed.

#### Scenario: The run produces a readable results table

- **WHEN** the probe finishes its configured runs
- **THEN** `results.md` holds a connection table, a DoH table, and a summary block, each rendering as valid markdown
