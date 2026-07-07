## ADDED Requirements

### Requirement: Each family action's button reflects only its own loading state

The system SHALL track which family action is currently in flight via a single `executingAction` discriminant on `FamiliesContext` (e.g. `create`, `invite`, `kick`, `revoke`, `leave`, `accept`, `reject`). A button SHALL show its loading label only when `executingAction` equals its own action. While any action is in flight, the other action buttons SHALL be disabled (greyed) but MUST NOT show a loading/spinner state that doesn't belong to them.

#### Scenario: Sending an invite only loads the invite button
- **WHEN** the user clicks "Send invite"
- **THEN** the "Send invite" button SHALL show "Sending…" and be disabled
- **THEN** any other family action button on screen SHALL be disabled but SHALL remain in its default (non-loading) label

#### Scenario: An in-flight action greys out the others without loading them
- **WHEN** any family action is in flight
- **THEN** the other action buttons SHALL be disabled
- **THEN** none of those other buttons SHALL display a loading label

#### Scenario: Buttons return to default after the action completes
- **WHEN** an action resolves (success or error) and `executingAction` returns to `null`
- **THEN** every action button SHALL return to its default label and enabled state (subject to its own validation)

## Notes

This replaces the original global `isExecuting` boolean, which made unrelated buttons (e.g. "Save changes" and "Send invite") enter the loading state together. The fix uses one `executingAction` discriminant rather than per-form local `useState` flags, because more than two actions share the surface (invite, kick, revoke, leave) and a single discriminant keeps them consistent. Note that "Edit family" / "Save changes" now lives on the separate Family Settings page (`FamilySettingsPage`), so it no longer sits next to "Send invite"; the discriminant still guarantees per-action loading wherever the buttons appear.
