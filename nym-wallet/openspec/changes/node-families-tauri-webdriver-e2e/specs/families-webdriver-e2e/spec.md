## ADDED Requirements

### Requirement: Native-webview e2e harness

The project SHALL provide a WebdriverIO test harness that drives the mock-wired Tauri binary through `tauri-driver` against the platform native webview. The harness MUST launch the actual desktop application (not a browser pointed at a dev server) and MUST NOT require a live chain or Rust IPC handlers, relying instead on the build-time mock providers.

#### Scenario: Harness launches the mock-wired binary

- **WHEN** the WebdriverIO suite starts
- **THEN** `tauri-driver` launches the mock-wired Tauri binary and a session attaches to its native webview
- **AND** the Family page is reachable within that session

#### Scenario: Unsupported platform is skipped, not failed

- **WHEN** the suite is invoked on macOS (no WKWebView driver)
- **THEN** the WebdriverIO/`tauri-driver` suite is skipped with a clear message rather than reported as a failure

### Requirement: Owner lifecycle journey

The harness SHALL replay the owner lifecycle journey end to end against the native webview: create a family, invite the self-controlled node, accept the invite from the operator tab, kick the member, and disband the family. The journey SHALL assert the same post-step DOM transitions verified by the Storybook owner-lifecycle flow.

#### Scenario: Owner create-to-disband completes

- **WHEN** the harness runs the owner lifecycle against the owner-persona mock build
- **THEN** creating a family reveals the owner management page
- **AND** the invited node appears as a pending invite, then as a joined member after acceptance
- **AND** kicking the member removes it, and disbanding returns the create-family entry point

### Requirement: Operator lifecycle journey

The harness SHALL replay the operator lifecycle journey end to end: accept an invite on one controlled node, leave that family, then reject an invite on another controlled node. The journey SHALL assert the same post-step DOM transitions verified by the Storybook operator-lifecycle flow, including that the reject-node invite group ends empty.

#### Scenario: Operator accept-leave-reject completes

- **WHEN** the harness runs the operator lifecycle against the operator-persona mock build
- **THEN** accepting the invite shows the current-family card with a leave action
- **AND** after leaving and rejecting the other node's invite, that node's invite group is empty

### Requirement: Selector and journey parity with Storybook

The WebdriverIO journeys SHALL target the same `data-testid` selectors and assert the same observable outcomes as the existing Storybook flow stories and Playwright specs, so the two suites verify equivalent behavior across the browser and native-webview environments. Confirmation dialogs that portal outside the page canvas SHALL be located by their global test ids, mirroring the Storybook `screen`-scoped queries.

#### Scenario: Equivalent assertions across environments

- **WHEN** a journey step is asserted in both the Playwright/Storybook suite and the WebdriverIO suite
- **THEN** both suites query the same `data-testid` and expect the same visible/absent outcome

### Requirement: CI execution on a supported platform

The WebdriverIO suite SHALL run in CI on a supported platform (Linux). The CI job MUST install the platform webdriver (`WebKitWebDriver`) and `tauri-driver`, build the mock-wired binary, and execute the suite; failures of any owner or operator journey MUST fail the job.

#### Scenario: CI runs the native-webview journeys

- **WHEN** CI runs on Linux for the change
- **THEN** the job installs `WebKitWebDriver` and `tauri-driver`, builds the mock-wired binary, and runs the WebdriverIO journeys
- **AND** the job fails if any owner or operator journey assertion fails
