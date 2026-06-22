## ADDED Requirements

### Requirement: Edit and Invite buttons reflect only their own loading state
The system SHALL track loading state independently for the "Save changes" and "Send invite" actions so that activating one does NOT change the state of the other button.

#### Scenario: Saving family details does not affect Send invite button
- **WHEN** the user clicks "Save changes"
- **THEN** the "Save changes" button SHALL show "Saving…" and be disabled
- **THEN** the "Send invite" button SHALL remain in its default state (not "Sending…")

#### Scenario: Sending invite does not affect Save changes button
- **WHEN** the user clicks "Send invite"
- **THEN** the "Send invite" button SHALL show "Sending…" and be disabled
- **THEN** the "Save changes" button SHALL remain in its default state (not "Saving…")

#### Scenario: Both buttons return to default after operation completes
- **WHEN** either operation resolves (success or error)
- **THEN** the corresponding button SHALL return to its default label and enabled state (if inputs are valid)
