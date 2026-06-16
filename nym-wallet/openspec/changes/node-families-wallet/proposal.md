## Why

Node Families is a new on-chain capability (see the `node-families-contract` spec) that lets a family owner group nodes under a single family and lets node operators join, leave, and respond to invites. The Nym Wallet currently has no surface for any of this. This change introduces a **Family Tab** in the wallet so the two personas — family owners and node operators — can drive the full family lifecycle from the UI, end to end. It delivers tickets NYM-1210 through NYM-1219.

## What Changes

**Family owner flows (NYM-1210–1215)**
- New **Family Tab**, always visible to any wallet account: shows a create-family entry point when the account owns no family, and the family management surface when it does.
- Create a family: attach the configured creation fee (`Config::create_family_fee`), set name + description, surface insufficient-balance and fee errors.
- Add/edit family **name** and **description**, with byte-length limits and input sanitisation, inline over-limit errors.
- Invite a node by **node ID**: triggers the contract invite (nonce/TTL via `validity_secs`), with confirmation; warns and does not send if the node is already in a family, does not exist, or already has a pending invite from this family.
- Manage **pending invites**: withdraw an active invite (confirmation prompt), dismiss/clear expired invites (confirmation), with list + contract state kept in sync.
- View the **member list** grouped by status: Pending / Joined / Rejected / Removed, with per-status empty states and refresh.
- **Kick** a member (confirmation prompt → contract `KickFromFamily`), moving the node to Removed.
- **Delete** an empty family (`DisbandFamily`); blocked with a clear error when members remain.

**Node operator flows (NYM-1216–1219)**
- View incoming **invites per node** (multi-node aware): family name, inviting owner, expiry/TTL; expired invites shown as non-actionable.
- **Accept** an invite (`AcceptFamilyInvitation`) → node moves to Joined. V1 acceptance is a pure membership record; owner-acts-for-node behaviour (where the family owner could perform actions on member nodes) is V2 per NYM-1217 and out of scope here.
- **Reject** an invite (`RejectFamilyInvitation`, confirmation) → no longer shown, node reflected as Rejected.
- **Leave** a family (`LeaveFamily`, confirmation) → removed from member list; can subsequently receive/accept new invites.

**Engineering scope**
- React **hooks** wrapping the contract surface (queries + execute msgs), each with a **mock** counterpart following the existing `src/context/<x>.tsx` ↔ `src/context/mocks/<x>.tsx` pattern.
- **Storybook** coverage on three levels: component states, composed pages, and full user-flow stories with simulated actions, all driven by the mocked hooks/contract data.
- **Tests**: Storybook interaction tests, Playwright end-to-end flows, and hook/integration tests against the mocks.
- UI implemented from **Figma** (designs supplied via Figma MCP during apply).

**Contract dependencies** (landing in a separate contract change; this branch rebases onto it before merge):
- **Edit name/description after creation** (NYM-1211): the contract spec has `CreateFamily` (carries name/description) and `UpdateConfig` (admin) but no `UpdateFamily` edit handler yet. The contract team will add it; the wallet builds against an assumed message shape and verifies it on rebase (see design.md Open Questions).

## Capabilities

### New Capabilities
- `node-families-owner`: Wallet behavior for the family-owner persona — Family Tab visibility, create/edit/delete family, invite/withdraw/clear invitations, and the status-grouped member list.
- `node-families-operator`: Wallet behavior for the node-operator persona — per-node invite viewing (with TTL/expiry), accept, reject, and leave.

### Modified Capabilities
<!-- None in the wallet's own spec set. The two contract gaps above belong to the
     `node-families-contract` capability in the root openspec project and must be
     resolved there (separate change); they are tracked here as dependencies, not deltas. -->

## Impact

- **Code**: new Family Tab pages/components under `src/pages` + `src/components`; new context/hooks in `src/context` (`families.tsx` owner, `familyInvites.tsx` operator or a combined `families.tsx`) with mocks in `src/context/mocks`; new types in `src/types`; new request/IPC bindings (Tauri) for the contract execute/query methods.
- **Mocks**: a faithful `node-families-contract` mock under `src/context/mocks` (provider + `families.fixtures.ts`) derived from `openspec/specs/node-families-contract/spec.md`, covering its full surface — Config, all data types, every execute msg and query (with pagination), enforced invariants, the typed error set, and emitted events. Follows the existing `mocks/bonding.tsx` convention.
- **Storybook**: new stories tree for components → pages → flows.
- **Tests**: Playwright e2e specs; Jest/RTL hook + integration tests against mocks; Storybook interaction tests.
- **Dependencies / blockers**: `UpdateFamily` edit lands in a separate contract change; this wallet branch will rebase onto that change before merge, and the edit path swaps from the mock to the real IPC binding at rebase time (verified per task 9.5). Creation fee is configurable on-chain (not a hardcoded 100 NYM); UI must read it from config.
- **External**: Figma designs (via Figma MCP) required during implementation — Nym 2.0 file `moIK1E6AaXhFz8lI1pZVrI`, board "Nym_Wallet – Node families added" (`1859:981`); per-frame mapping in design.md "Design Source (Figma)".
