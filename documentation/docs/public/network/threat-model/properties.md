---
title: Unlinkability properties
description: The two unlinkability properties the threat model measures: request-identity unlinkability (P1) and request-request unlinkability (P2), their asymmetry, and the fragile pseudonymous-profile state most configurations land in.
url: https://nym.com/docs/network/threat-model/properties
---

# Unlinkability properties

Every configuration is measured against two properties. P1 asks whether a single
request can be attributed to you. P2 asks whether two requests can be tied to the
same client. The comparison matrix and the per-actor assessments throughout the
network docs are verdicts on these two properties.

## The asymmetry matters

P1 and P2 are not symmetric. A P1 failure across requests implies a P2 failure,
because requests attributable to one identity are thereby linkable to each other.
The converse does not hold: requests can be linkable to each other (P2 fails)
without being attributable to a person (P1 holds). This is the pseudonymous
profile, and it is fragile.

One attributed request anywhere in a pseudonymous profile retroactively
attributes the whole profile. This is why so many configurations carry a "given
per-request rotation" caveat: without it, the profile builds, and a single slip
collapses it.
