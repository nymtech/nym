# family-countdown-timer Specification

## Purpose
TBD - created by archiving change node-families-ui-polish. Update Purpose after archive.
## Requirements
### Requirement: Countdown timer ticks live

The system SHALL provide a `nowSecs` value on `FamiliesContext` that advances once per second, so that all pending-invite expiry countdowns progress in real time. `nowSecs` is produced by the `useNowSecs` hook (`src/hooks/useNowSecs.ts`), which owns the 1-second interval and cleans it up on unmount. Countdown consumers include the unified members table (`FamilyMembersTable`) on the owner surface and the invite cards (`InviteCard`) on the Invites surface.

#### Scenario: Timer advances after mount
- **WHEN** the Family page is mounted and a pending invite is shown with a remaining time of "in 5 min"
- **THEN** after roughly 60 seconds the displayed remaining time SHALL have decreased by about 1 minute

#### Scenario: Timer cleans up on unmount
- **WHEN** the component owning `useNowSecs` unmounts
- **THEN** the interval SHALL be cleared and no further state updates occur

#### Scenario: Expired invite shows the correct label
- **WHEN** an invitation's `expires_at` is in the past relative to the current `nowSecs`
- **THEN** the expiry display SHALL show "Expired" rather than a negative duration

#### Scenario: Countdown reflects the contract's configured validity
- **WHEN** a pending invite is shown
- **THEN** its remaining time SHALL be derived from the invitation's `expires_at` (set from the contract's configured validity period), not from any client-side guess

