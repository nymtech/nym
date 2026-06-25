# families-app-e2e Specification

## Purpose
TBD - created by archiving change node-families-tauri-webdriver-e2e. Update Purpose after archive.
## Requirements
### Requirement: Primary e2e against the mock-wired dev server

The project SHALL provide a Playwright suite that drives the wallet app served by the dev server (`http://localhost:9000`) with the families mock flag enabled, exercising the Family page within the real app shell and router (not a Storybook iframe). The suite MUST run cross-platform (including macOS) and MUST NOT require a live chain or Rust IPC. It SHALL reuse the existing journey `data-testid` selectors.

#### Scenario: Suite drives the app shell, not Storybook

- **WHEN** the Playwright suite starts
- **THEN** it launches (or reuses) the mock-wired dev server on `:9000` and navigates to the Family page within the app router
- **AND** the page renders backed by `MockFamiliesContextProvider` with no IPC call

#### Scenario: Runs on the developer's platform

- **WHEN** the suite is run locally on macOS
- **THEN** it executes the journeys in a real browser and reports pass/fail (no platform skip)

### Requirement: Owner lifecycle journey

The primary suite SHALL replay the owner lifecycle end to end against the app shell: create a family, invite the self-controlled node, accept the invite from the operator tab, kick the member, and disband the family, asserting the same post-step DOM transitions as the Storybook owner-lifecycle flow.

#### Scenario: Owner create-to-disband completes

- **WHEN** the suite runs the owner lifecycle against the owner-persona mock build
- **THEN** creating a family reveals the owner management page
- **AND** the invited node appears as a pending invite, then as a joined member after acceptance
- **AND** kicking the member removes it, and disbanding returns the create-family entry point

### Requirement: Operator lifecycle journey

The primary suite SHALL replay the operator lifecycle end to end: accept an invite on one controlled node, leave that family, then reject an invite on another controlled node, asserting the same post-step DOM transitions as the Storybook operator-lifecycle flow, including that the reject-node invite group ends empty.

#### Scenario: Operator accept-leave-reject completes

- **WHEN** the suite runs the operator lifecycle against the operator-persona mock build
- **THEN** accepting the invite shows the current-family card with a leave action
- **AND** after leaving and rejecting the other node's invite, that node's invite group is empty

### Requirement: Selector and journey parity across suites

All e2e suites (primary Playwright, and the optional native-webview leg) SHALL target the same `data-testid` selectors and assert the same observable outcomes as the existing Storybook flow stories, so they verify equivalent behavior across environments. Confirmation dialogs that portal outside the page canvas SHALL be located by their global test ids, mirroring the Storybook `screen`-scoped queries.

#### Scenario: Equivalent assertions across environments

- **WHEN** a journey step is asserted in more than one suite
- **THEN** every suite queries the same `data-testid` and expects the same visible/absent outcome

### Requirement: Optional native-webview validation leg

The project SHALL provide an optional WebdriverIO + `tauri-driver` leg that launches the packaged Tauri binary and replays the owner and operator journeys against the platform native webview, following the Tauri WebDriver-in-CI flow (Ubuntu runner, `xvfb-run` headless display, `webkit2gtk-driver` + `tauri-driver`). The leg MUST run in CI on a supported platform (Linux) and MUST skip — not fail — on unsupported platforms (macOS) or when `tauri-driver`/`webkit2gtk-driver` is absent.

#### Scenario: Native leg validates the binary in CI

- **WHEN** the native leg runs on the Linux CI runner
- **THEN** it installs `webkit2gtk-driver` and `tauri-driver`, builds the mock-wired binary, and replays the owner and operator journeys under `xvfb-run`
- **AND** a journey failure is reported (non-blocking while the leg is stabilizing)

#### Scenario: Skip-not-fail off-platform

- **WHEN** the native leg is invoked on macOS or without the required drivers
- **THEN** it is skipped with a clear message rather than reported as a failure

### Requirement: Optional sandbox real-IPC read smoke

The project MAY provide a read-only smoke that exercises the real `FamiliesContextProvider` + `src/requests/families.ts` against the node-families contract deployed to sandbox, validating the IPC wiring the mock stands in for. The smoke SHALL be read-only (no create/invite/kick/disband against shared sandbox) and SHALL assert rendered shape rather than exact contents (or pin a known family id), and MUST be separate from and non-blocking relative to the deterministic mock suites.

#### Scenario: Sandbox read smoke renders real data

- **WHEN** the read smoke runs against the sandbox-connected app
- **THEN** the Family page renders the sandbox family/member via real IPC without performing any state-changing transaction
- **AND** a failure does not block the primary mock suites

