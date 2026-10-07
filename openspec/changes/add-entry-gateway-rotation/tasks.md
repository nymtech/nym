# Tasks: Add Entry-Gateway Rotation

## 0. Diagnostic gate (do first)

- [ ] 0.1 Confirm the common-mode exit failures are the entry gateway, not an over-short per-exit timeout: pin an IPR (original 60 s / 15 s budget) and/or raise `ipr_attempt_timeout` and check whether any exit connects. Build this change only if the entry gateway is confirmed as the fault.

## 1. Re-registration path

- [ ] 1.1 Add a `start_nym_client` variant (or parameter) that registers with a specific/excluded entry gateway, so escalation can re-register excluding the failed one
- [ ] 1.2 Tear down the failed base client cleanly before re-registering (sticky per-`client_id` registration; confirm a fresh registration does not collide with stored state)

## 2. Escalation loop

- [ ] 2.1 Wrap the exit-rotation loop in `WasmTunnel::new` with an outer entry-rotation loop, bounded by `ipr_max_entry_attempts` (default 3)
- [ ] 2.2 Escalate to entry rotation only after the exit-attempt bound is reached on the current entry
- [ ] 2.3 Exclude the just-failed entry gateway from the next selection
- [ ] 2.4 On exhaustion of both bounds, fail with a clear error naming the attempted entries and exits

## 3. Tuning

- [ ] 3.1 Add `ipr_max_entry_attempts` (default 3) to `TuningOpts` + builder + JS `SetupOpts`

## 4. Tests

- [ ] 4.1 All exits fail on entry A, entry B healthy: tunnel establishes after entry rotation. Record the selected entries and assert the rotated entry is not A (recovery alone can pass by retrying A before eventually choosing B, so assert exclusion explicitly)
- [ ] 4.2 Both bounds exhausted: fails fast with a clear error, no infinite cycling
- [ ] 4.3 Exit succeeds on the first entry: no entry rotation occurs

## 5. Out of scope

- Rotating the entry gateway on an established tunnel (establishment-time only).
- Certain fault attribution; the trigger is the repeated-common-mode-failure heuristic.
