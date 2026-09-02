## Context

`gateway::select` (in `nym-sdk-session`) picks a gateway per `GatewaySpec`
(`Identity` / `Country` / `Random`) and role, with a single optional `exclude`
identity used today only so a two-hop tunnel's exit is distinct from its entry.
`register_two_hop_inner` selects the entry, then the exit excluding the entry.

The example bring-up path `common::connect` (in `smoldvpn/examples`) already gates
on `Tunnel::await_established` and, on failure, invalidates and re-registers the
implicated hop(s) with a bounded, escalating retry: the first `exit: down`
re-registers the exit; a *fresh* exit still down implicates the entry and
re-registers it too.

The gap: re-registering an implicated entry re-selects with the same
`GatewaySpec`. With `Random` that can re-pick the same non-forwarding node; with a
pinned `Identity` it always re-picks it. Selection has no way to *avoid* a set of
known-bad entries. Reproduced against a sandbox entry gateway that passes its own
handshake but never forwards the tunnelled exit handshake.

## Goals / Non-Goals

**Goals:**
- Let selection avoid a *set* of gateway identities (generalizing the single
  `exclude`), so implicated entries can be skipped.
- Give the retry a way to move a **substitutable** entry (`Random`/`Country`) off a
  non-forwarding gateway deterministically, not by luck.
- Preserve pin semantics: a pinned `--entry <identity>` is **never** silently
  switched — it is retried and then fails on the attempt bound.

**Non-Goals:**
- Detecting/avoiding a broken exit-gateway *egress* (a node that completes the exit
  handshake but black-holes traffic). `await_established` proves only the
  handshake, so this needs a separate through-tunnel data probe — out of scope.
- Any change to the credential / fetcher / ticket-spend path.
- Health scoring or persistent reputation of gateways across sessions.

## Decisions

**1. `exclude: Option<&PublicKey>` → `exclude: &[ed25519::PublicKey]`.**
Set membership (`exclude.contains(id)`) replaces equality. A slice (not a
`HashSet`) is enough — the exclusion set is tiny (bounded by the retry count).
Every match arm already calls the `excluded` predicate, so only the predicate and
the call sites change. Alternative (keep `Option`, add a second `avoid` param):
rejected — two overlapping "don't pick this" inputs are more confusing than one
set.

**2. Mechanism in the callee, policy in the caller.** `select` /
`register_two_hop_avoiding_entries` only *exclude what they are given*; they do not
inspect the spec to decide whether exclusion is allowed. `common::connect` decides
whether an implicated entry is eligible for exclusion (only `Random`/`Country`) and
populates the set accordingly. This keeps the SDK API policy-free and testable, and
puts the "respect an explicit pin" rule where the CLI intent is known.

**3. Pinned identity is never substituted.** Two independent guarantees:
- `select`'s `Identity` arm already returns `SameGatewaySelected` if the pinned id
  is in the exclude set — it never falls back to another gateway.
- `connect` never adds a pinned entry to the avoid set, so re-selection keeps the
  pin, retries it, and fails on the bound with the existing bounded-attempts error.

**4. New method, stable existing API.** Add
`Session::register_two_hop_avoiding_entries(entry, exit, avoid_entries)`;
`register_two_hop` and `register_two_hop_quic` keep their signatures and delegate
with an empty `avoid_entries`. `register_two_hop_inner` grows one `&[PublicKey]`
parameter applied to the entry selection. Alternative (breaking-change the existing
signature): rejected — needless churn for callers that don't retry.

**5. Escalation unchanged; only the recovery action is added.** The retry's
blame logic (`prev_exit_down && status.entry` ⇒ entry implicated) is already in
place. This change only adds: on that condition, *if the entry spec is
substitutable*, add `reg.entry.gateway_identity` to the avoid set passed to the
next registration.

**6. Test the exclusion without a live topology.** The existing selection tests run
over an empty node set (constructing a full `NymNodeDescriptionV2` was deemed
impractical). To assert "`Random` skips an excluded set" we need ≥1 eligible node,
so add a small test helper that builds a minimal WireGuard-capable described node
(following the `nym-gateway-probe` construction pattern) keyed by a chosen
identity. The test builds two eligible nodes `{a, b}`, selects `Random` excluding
`[a]` repeatedly, and asserts the result is always `b`; and that excluding both
yields `NoWireguardGateway`. If minimal-node construction proves too heavy, fall
back to extracting the eligibility filter (identity + `wg_capable` + `quic_ok` +
`!excluded`) into a pure helper and unit-testing that.

## Risks / Trade-offs

- [A substitutable pool with only broken entries still can't recover] → The retry
  exhausts attempts and surfaces the bounded-attempts error; exclusion can shrink
  the candidate set to empty, which returns a clean `NoWireguardGateway` instead of
  re-picking a bad node. Acceptable — it fails clearly rather than silently looping.
- [Excluding a good entry wastes its slot] → Only entries implicated by a
  *fresh-exit-still-down* signal are excluded, and only for substitutable specs, so
  a healthy entry is not excluded on a first stale-exit failure.
- [Minimal-node test helper drifts from the real described-node shape] → Keep the
  helper minimal and local to the test module; it exercises only the fields
  `select` inspects (`wg_capable`, identity), not the full model.

## Migration Plan

Additive and backward-compatible: the `select` signature change is `pub(crate)`
(no external callers); the new `Session` method is additive; existing
`register_two_hop*` behavior is unchanged. No data migration, no config change, no
rollback concerns.

## Open Questions

- None. (Resolved: the `Country`-spec exclusion is **not** separately tested — the
  `Random`-skips-excluded test plus the existing pinned-exclusion test cover the
  mechanism, since `Country` and `Random` share the same `!excluded` candidate
  filter.)
