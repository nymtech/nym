## Capability

`family-nav-invite-badge` — Visual indicator on the Family nav item when the user's controlled node has pending family invitations.

## Requirements

- **R1** When the wallet loads, if the currently connected account controls a bonded node that has one or more non-expired pending family invitations, the Family entry in the sidebar navigation MUST display a visual indicator (dot badge).
- **R2** The indicator MUST disappear when there are no longer any pending invitations (e.g. after the user accepts, rejects, or the invite expires).
- **R3** The indicator MUST show a dot only — not a count number — to keep the nav uncluttered.
- **R4** If the account has no bonded node, or the pending invite query has not yet resolved, no indicator is shown (silent, no loading spinner in nav).
- **R5** The indicator MUST NOT require the user to have visited the `/family` route first — it reflects live data from initial page load.
- **R6** The implementation MUST NOT move `FamiliesContextProvider` above the `/family` route boundary.

## Out of Scope

- Badge for pending invitations sent by an owner (outbound pending invites) — this feature covers inbound operator invites only
- Numeric badge count
- Persistent notification history or unread tracking
