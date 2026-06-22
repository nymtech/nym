## ADDED Requirements

### Requirement: Countdown timer ticks live
The system SHALL update `nowSecs` in `FamiliesContext` on a 1-second interval so that all pending-invite expiry countdowns displayed in `PendingInvitesList`, `MemberList`, and `InviteCard` progress in real time.

#### Scenario: Timer advances after mount
- **WHEN** the Family page is mounted and a pending invite is shown with "in 5 min"
- **THEN** after 60 seconds the displayed time SHALL have decreased by approximately 1 minute

#### Scenario: Timer cleans up on unmount
- **WHEN** the Family page is unmounted
- **THEN** the interval SHALL be cleared and no further state updates occur

#### Scenario: Expired invite shows correct label
- **WHEN** `expiresAt` is in the past relative to the current `nowSecs`
- **THEN** the expiry display SHALL show "Expired" not a negative duration
