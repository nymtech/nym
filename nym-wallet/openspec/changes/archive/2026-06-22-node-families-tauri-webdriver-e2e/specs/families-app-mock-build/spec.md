## ADDED Requirements

### Requirement: Dedicated mock entry, gated by a build flag

The wallet app SHALL provide a dedicated mock entry (a separate webpack entry + generated HTML) that mounts the Family page with the mock app bootstrap (`MockMainContextProvider`) and mock families provider (`MockFamiliesContextProvider`), requiring no Tauri runtime and no login. A build flag (`WALLET_MOCK_FAMILIES`, default off) SHALL gate whether the mock entry and its HTML are built at all, so a production build never includes the mock entry or any families mock-engine code. The real `/family` route and `FamilyPage` component SHALL be left unchanged so the page stays backed by `FamiliesContextProvider` in production and remains importable in isolation.

#### Scenario: Mock build wires the mock providers

- **WHEN** the app is built with the families mock flag enabled
- **THEN** the mock entry renders `FamilyPage` wrapped in `MockFamiliesContextProvider` (and the app shell in `MockMainContextProvider`)
- **AND** the page resolves family reads from the in-memory mock engine without any Tauri IPC call or login

#### Scenario: Production build excludes the mock entry

- **WHEN** the app is built with the families mock flag disabled (default)
- **THEN** no mock entry or `main.mock.html` is produced and the bundle contains no families mock-engine code
- **AND** the real `/family` route still renders `FamilyPage` wrapped in `FamiliesContextProvider`

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
