# Design: Add Entry-Gateway Rotation

## Context

Builds on `add-ipr-connect-retry`, which rotates the exit IPR. Live testing surfaced the
gap: exit rotation cannot recover from a fault upstream of the exit. Against entry
gateway `koutakou.cloud`, the client registered, discovered 606 exits, and rotated
through five, every one timing out with no response. Five independent exits failing
identically is a common-mode signature, and the common element is the entry gateway and
the send path through it.

## The diagnostic came back: it was not the entry gateway

The observation that prompted this change was later traced to a different cause. Two
explanations fit the original log:

1. The entry gateway or send path is the fault. Exit rotation is futile; entry rotation
   is the fix. This change.
2. The per-exit timeout strangled the v10-then-v9 fallback. The exits logged `v10
   connect timed out; retrying v9`, but the single outer per-exit timeout fired before
   v9 could send, so exits reachable only on v9 were abandoned.

Further testing settled it as explanation 2. The same failure appeared against three
different entry gateways, and a pinned exit connected sub-second, so the entry gateways
were not the fault; the outer timeout was. The fix landed in `add-ipr-connect-retry`:
drop the outer per-exit timeout and let the per-version fallback run.

So this change is no longer motivated by that incident. It remains a genuine resilience
feature, because a truly dead entry gateway is possible and exit rotation cannot recover
from it, but it is now speculative rather than urgent. Build it only if a genuinely
dead-entry case is observed after the fallback fix, and only after confirming exit
rotation cannot already recover.

## Why entry rotation is a separate, heavier tier

Exit rotation is cheap: the exit IPR is chosen per attempt from a discovery list, and
switching is just picking the next address. Entry-gateway rotation is not symmetric. The
base client registers with one entry gateway at startup, and registration derives a
shared key with that gateway. An established client cannot be repointed to a different
gateway; it must register afresh. So entry rotation means tearing down the base client
and re-registering, which is why it is the escalation tier, reached only after exit
rotation has exhausted, not the first response.

## Trigger and bounds

- Escalate after the exit-attempt bound (`ipr_max_attempts`, default 5, or a smaller
  common-mode threshold such as 3) of distinct exits all fail on the current entry.
- Bound entry rotation with `ipr_max_entry_attempts` (default 3) so a network-wide
  outage terminates. Worst case is `ipr_max_entry_attempts` entries times the exit
  budget, which must stay within a sane establishment ceiling.
- Exclude the failed entry from the next selection; reuse the base client's existing
  performance-weighted gateway selection for the rest.

## Open questions

- The exact common-mode threshold. Using the full `ipr_max_attempts` (5) before
  escalating is simplest; a smaller threshold escalates sooner but risks re-registering
  when the exits were merely slow. The threshold interacts directly with the per-exit
  budget fix above.
- Whether a re-registration cleanly replaces the stored gateway for a `client_id`, or
  needs a fresh `client_id`. To be confirmed against the storage and registration code.
