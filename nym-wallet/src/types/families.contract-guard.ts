/* eslint-disable @typescript-eslint/naming-convention, @typescript-eslint/no-unused-vars */
//
// Contract-shape guard (tasks.md 2.3).
//
// Locks the wallet-facing `src/types/families.ts` types against the
// contract-truth types generated from the Rust `node-families-contract` by
// `tools/ts-rs-cli` (committed under `ts-packages/types/src/types/rust/`).
//
// This file emits no runtime code — it is a set of compile-time assertions
// checked by `tsc` / the webpack ForkTsChecker. If the contract changes shape
// and the generated `*.ts` are regenerated, any field added / removed / renamed
// / retyped on a 1:1 type below breaks the build here, forcing a conscious
// reconciliation of `families.ts` (and the mock, which mirrors it).
//
// Imports are by relative path on purpose: the wallet resolves `@nymproject/types`
// to the package's built `dist`, but the generated source is the source of truth
// for drift, so we read it directly.

import type { NodeFamily as ContractNodeFamily } from '../../../ts-packages/types/src/types/rust/NodeFamily';
import type { FamilyConfig as ContractFamilyConfig } from '../../../ts-packages/types/src/types/rust/FamilyConfig';
import type { FamilyMembership as ContractFamilyMembership } from '../../../ts-packages/types/src/types/rust/FamilyMembership';
import type { FamilyInvitation as ContractFamilyInvitation } from '../../../ts-packages/types/src/types/rust/FamilyInvitation';
import type { NodeFamilyMembershipResponse as ContractMembershipResponse } from '../../../ts-packages/types/src/types/rust/NodeFamilyMembershipResponse';
import type { PendingFamilyInvitationDetails as ContractPendingDetails } from '../../../ts-packages/types/src/types/rust/PendingFamilyInvitationDetails';
import type { PastFamilyMember as ContractPastMember } from '../../../ts-packages/types/src/types/rust/PastFamilyMember';

import type {
  NodeFamily,
  FamilyConfig,
  FamilyMembership,
  FamilyInvitation,
  NodeFamilyMembershipResponse,
  PendingFamilyInvitationDetails,
  PastFamilyMember,
} from './families';

// --- type-equality machinery ------------------------------------------------

type Equal<X, Y> = (<T>() => T extends X ? 1 : 2) extends <T>() => T extends Y ? 1 : 2 ? true : false;
type Expect<T extends true> = T;

// --- 1:1 types (must stay structurally identical to contract) ---------------
//
// These flow through the Tauri command layer unchanged, so the wallet type and
// the generated contract type must match exactly.

type _FamilyMembership = Expect<Equal<FamilyMembership, ContractFamilyMembership>>;
type _FamilyInvitation = Expect<Equal<FamilyInvitation, ContractFamilyInvitation>>;
type _MembershipResponse = Expect<Equal<NodeFamilyMembershipResponse, ContractMembershipResponse>>;
type _PendingDetails = Expect<Equal<PendingFamilyInvitationDetails, ContractPendingDetails>>;
type _PastMember = Expect<Equal<PastFamilyMember, ContractPastMember>>;

// --- intentionally normalised in the Rust IPC layer -------------------------
//
// The following diverge by design (see operations/families/query.rs and the
// `families.ts` header). They are asserted on the *non-translated* fields only;
// the translated fields are excluded so the guard still catches drift on the
// rest:
//
//   * `NodeFamily.paid_fee` / `FamilyConfig.create_family_fee`:
//       base-denom contract `Coin` ({denom,amount}) -> display `DecCoin`.
//   * Paginated responses ({family_id|node_id, members|invitations, ...}) ->
//       the uniform `FamilyPagedResponse<T> = { items, start_next_after }`.
//   * `FamilyInvitationStatus` (cw_serde tagged) -> the `{ kind, at }` union.

type _NodeFamilySansFee = Expect<Equal<Omit<NodeFamily, 'paid_fee'>, Omit<ContractNodeFamily, 'paid_fee'>>>;
type _FamilyConfigSansFee = Expect<
  Equal<Omit<FamilyConfig, 'create_family_fee'>, Omit<ContractFamilyConfig, 'create_family_fee'>>
>;
