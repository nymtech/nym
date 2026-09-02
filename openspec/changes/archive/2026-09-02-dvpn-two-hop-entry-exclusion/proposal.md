## Why

On a two-hop bring-up, the entry gateway can pass its own (direct) WireGuard
handshake while silently failing to **forward** the tunnelled exit handshake — the
exit then never establishes (`entry: up, exit: down`) even though the exit gateway
is healthy. This was reproduced against a misconfigured sandbox entry gateway: the
exit init is sent and retransmitted but no reply ever returns. The current
establish-gated retry re-registers only the exit, so it never escapes a
non-forwarding entry; with random selection it recovers only by luck, and with a
pinned entry it cannot recover at all. Selection has no way to *avoid* an entry
gateway once implicated, so recovery is left to chance.

## What Changes

- Generalize gateway selection's single `exclude` to a **set** of excluded
  identities, so more than one gateway (e.g. several implicated entries) can be
  avoided in one selection. This subsumes the existing "exclude the entry from
  exit selection" behavior.
- Add a two-hop registration path that takes a set of **entry** identities to
  avoid, so an implicated non-forwarding entry can be excluded from re-selection
  rather than re-picked at random.
- Selection never *silently substitutes* a pinned identity: an excluded pinned
  `Identity` fails with the existing distinct-gateways error; only substitutable
  specs (`Random`/`Country`) skip excluded candidates.
- Harden the example bring-up retry (`common::connect`) to **escalate blame** to
  the entry: when a *freshly registered* exit is still down while the entry's own
  handshake is up, the entry is implicated. For a substitutable entry spec the
  implicated entry is added to the avoid set (and re-selection moves off it); a
  pinned `--entry <identity>` is **never switched** — it is retried and then fails
  after the attempt bound.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `dvpn-session`: gateway selection accepts a *set* of excluded identities (not a
  single one), and two-hop registration exposes an entry-avoidance path; a pinned
  identity is never substituted when excluded.
- `dvpn-tools`: the shared example bring-up retry escalates blame to the entry when
  a fresh exit still fails, excludes implicated substitutable entries from
  re-selection, and never switches a pinned entry (retry, then fail on the bound).

## Impact

- `sdk/rust/nym-sdk-session/src/gateway.rs` — `select`'s `exclude` parameter
  becomes a slice; the `excluded` predicate uses set membership. In-crate call
  sites and unit tests updated; a new unit test asserts a `Random` selection skips
  an excluded set.
- `sdk/rust/nym-sdk-session/src/session.rs` — `register_two_hop_inner` gains an
  `avoid_entries` parameter threaded to the entry `select`; a new public
  `register_two_hop_avoiding_entries` method; existing `register_two_hop` /
  `register_two_hop_quic` unchanged (delegate with an empty set).
- `smoldvpn/examples/common/mod.rs` — `connect` accumulates implicated entry
  identities and passes them to the new registration path, gated on the entry
  spec being substitutable (`Random`/`Country`).
- No change to the credential/fetcher/ticket-spend path. Does not address broken
  exit-gateway *egress* (a separate data-plane fault the establishment gate cannot
  detect).
