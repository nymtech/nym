## ADDED Requirements

### Requirement: Build-time mock provider selection

The wallet app SHALL select the Node Families context provider at build time based on a single mock flag. When the flag is enabled, the Family page SHALL be backed by `MockFamiliesContextProvider`; otherwise it SHALL be backed by the Tauri IPC-backed `FamiliesContextProvider`. The selection MUST be resolved by a compile-time constant so that, when the flag is disabled, mock modules (`src/context/mocks/**` for families) are eliminated from the bundle by tree-shaking and never reach a production build.

#### Scenario: Mock build wires the mock provider

- **WHEN** the app is built with the families mock flag enabled
- **THEN** the Family route renders `FamilyPage` wrapped in `MockFamiliesContextProvider`
- **AND** the page resolves family reads from the in-memory mock engine without any Tauri IPC call

#### Scenario: Production build excludes mock code

- **WHEN** the app is built with the families mock flag disabled (default)
- **THEN** the Family route renders `FamilyPage` wrapped in the real `FamiliesContextProvider`
- **AND** the produced bundle contains no families mock-engine code

### Requirement: Deterministic seeded fixtures in the mock build

The mock-wired build SHALL seed the families mock store from the same deterministic fixtures used by the Storybook flow stories, so that a given launch presents a reproducible starting state for each persona. The build SHALL expose, without a live chain or Rust handlers, an owner-persona entry state and an operator-persona entry state equivalent to `buildOwnerFlowStore` and `buildOperatorFlowStore`.

#### Scenario: Owner persona entry state

- **WHEN** the mock build launches in the owner-persona configuration
- **THEN** the Family page opens with no existing family and the create-family entry point is reachable
- **AND** the self-controlled flow node is available to invite

#### Scenario: Operator persona entry state

- **WHEN** the mock build launches in the operator-persona configuration
- **THEN** the Node invites tab presents the seeded pending invitations on the operator's controlled nodes

### Requirement: Family page reachable in the Tauri app shell

The mock-wired build SHALL mount the Family page within the production app shell (the real router and chrome), not a Storybook iframe, and the page SHALL be reachable through normal in-app navigation. The page and its interactive elements SHALL expose the same `data-testid` selectors as the Storybook stories so a single set of journey selectors works in both environments.

#### Scenario: Navigating to the Family page in the app

- **WHEN** the mock-wired app is loaded and the user navigates to the Family section
- **THEN** the `family-page` element is rendered inside the app shell
- **AND** the owner and operator tabs (`family-tab-owner`, `family-tab-operator`) are present with the same test ids used in Storybook
