## ADDED Requirements

### Requirement: Establish-gated two-hop bring-up retries and escalates blame

The shared example bring-up (`connect`) SHALL bring the tunnel up, gate on per-hop
WireGuard establishment within a bound, and on failure invalidate and re-register the
implicated hop(s), retrying up to a fixed maximum number of attempts before surfacing
a clear error naming that the tunnel did not establish after that many attempts.

Blame attribution SHALL escalate. A first `exit: down` SHALL re-register only the
exit (the common stale-cached-exit case). A **freshly registered** exit that is still
down while the entry's own (direct) handshake is up SHALL implicate the **entry** —
because an entry gateway can complete its own handshake yet fail to forward the
tunnelled exit handshake. When the entry spec is substitutable (`Random` or
`Country`), an implicated entry SHALL be excluded from re-selection so the retry
moves off a non-forwarding entry. When the entry spec is a pinned identity
(`--entry <identity>`), the entry SHALL never be switched: it is retried and then
fails on the attempt bound.

#### Scenario: Stale exit recovers by re-registering the exit
- **WHEN** the entry establishes but a cached exit does not, and a fresh exit
  registration then establishes
- **THEN** the tunnel comes up without re-selecting the entry

#### Scenario: Non-forwarding substitutable entry is excluded on escalation
- **WHEN** the entry spec is random, the entry's own handshake is up, and a freshly
  registered exit is still down
- **THEN** the entry is implicated, added to the re-selection avoid set, and the next
  attempt selects a different entry

#### Scenario: Pinned non-forwarding entry is retried and then fails
- **WHEN** the entry is pinned by identity and does not forward the exit handshake
- **THEN** the retry never switches to a different entry, and after the attempt bound
  the bring-up fails with the did-not-establish error

#### Scenario: Bounded attempts surface a clear error
- **WHEN** the tunnel does not establish after the maximum number of attempts
- **THEN** the bring-up returns an error stating it failed to establish after that
  many attempts, including the last per-hop establishment status
