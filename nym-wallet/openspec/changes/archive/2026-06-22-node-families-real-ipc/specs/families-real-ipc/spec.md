## ADDED Requirements

### Requirement: Tauri command layer for the node-families contract

The wallet SHALL implement the Tauri commands the frontend invokes for node families — the 9 execute commands (`create_family`, `update_family`, `disband_family`, `invite_to_family`, `revoke_family_invitation`, `kick_from_family`, `accept_family_invitation`, `reject_family_invitation`, `leave_family`) and the query commands (`get_family_by_id`, `get_family_by_owner`, `get_family_membership`, `get_family_config`, `get_family_members_paged`, `get_pending_invitations_for_family_paged`, `get_pending_invitations_for_node_paged`, `get_past_invitations_for_family_paged`, `get_past_members_for_family_paged`). Each command MUST delegate to the existing `validator-client` node-families traits using the connected account's client, and MUST be registered in the Tauri `invoke_handler`. Command names and argument shapes MUST match `src/requests/families.ts`.

#### Scenario: Every frontend command resolves on the Rust side

- **WHEN** the frontend invokes any of the 18 node-families commands
- **THEN** a registered Rust handler executes it against the node-families contract via the validator client
- **AND** no command falls through to "command not found"

#### Scenario: Execute returns a parsed family transaction result

- **WHEN** an execute command (e.g. `create_family`) succeeds on chain
- **THEN** the command returns a `FamilyTxResult` whose shape matches the frontend type, including `family_events` derived from the transaction (not fabricated)

#### Scenario: Queries return the frontend-typed shapes

- **WHEN** a query command runs against the contract
- **THEN** it returns data matching the corresponding `src/types/families.ts` shape (`NodeFamily`, membership, paged response with cursor)

### Requirement: Real provider replaces the mock in the running app

The production `/family` route SHALL render `FamilyPage` backed by the real `FamiliesContextProvider` (Tauri IPC), not the mock. The provider SHALL derive `controlledNodeIds` from the connected account's bonded nodes (removing the current empty-stub), so the operator view reflects nodes the account actually controls. The mock entry SHALL remain available for the offline e2e suite but MUST NOT back the production route.

#### Scenario: Production app shows live family data

- **WHEN** a signed-in account opens the Family page in the production app connected to a network with the contract
- **THEN** the page renders that account's family / invites from on-chain queries via the real provider
- **AND** the operator tab lists the account's controlled bonded nodes

#### Scenario: Mock stays isolated to e2e

- **WHEN** the production build is produced
- **THEN** the `/family` route uses `FamiliesContextProvider` and the mock provider/entry is excluded (per the mock-build gate)

### Requirement: Journeys pass against the sandbox contract

The owner and operator journeys SHALL be verifiable against the node-families contract on **sandbox** through the real provider. A read-only smoke MUST confirm queries render the known sandbox family/member via real IPC. Execute flows, when exercised, MUST run against a dedicated funded sandbox test account and MUST NOT depend on or corrupt unrelated shared state; the suite SHALL be iterated until the targeted journeys pass.

#### Scenario: Sandbox read smoke

- **WHEN** the app is connected to sandbox and the Family page loads
- **THEN** the real queries return and render the sandbox family/member without any state-changing transaction

#### Scenario: Sandbox execute flow (guarded)

- **WHEN** an execute journey runs against a funded sandbox test account
- **THEN** the on-chain state change is reflected back through the real queries in the UI
- **AND** the flow targets only that test account's family/nodes
