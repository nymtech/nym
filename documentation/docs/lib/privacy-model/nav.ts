// Data-derived navigation (Workstream E).
//
// The Nextra sub-navs for the configurations and the worked examples are
// projections of the typed scenario data, not independently hand-maintained
// lists. This module is the single source of the derivation; nav.test.ts asserts
// that the committed `_meta.json` files equal these derivations (keys, order and
// labels), so adding a scenario to the data and forgetting the nav fails the
// test rather than silently drifting.
//
// The matrix (ConfigMatrix) and the per-configuration diagrams already read the
// same GENERIC_SCENARIOS array, so a single data edit flows through matrix,
// diagram and nav together.

import { GENERIC_SCENARIOS } from "./examples/generic";

/**
 * The configurations sub-nav (`network/threat-model/configurations/_meta.json`),
 * derived from GENERIC_SCENARIOS: ordered by the array, labelled by shortTitle,
 * with a "(recommended)" suffix on the recommended configuration.
 */
export function configNavMeta(): Record<string, string> {
  const out: Record<string, string> = {};
  for (const s of GENERIC_SCENARIOS) {
    out[s.id] = s.recommended ? `${s.shortTitle} (recommended)` : s.shortTitle;
  }
  return out;
}

/**
 * The worked-examples sub-nav (`network/threat-model/examples/_meta.json`). The
 * examples live in separate datasets (wallet.ts, messaging.ts, browsing.ts)
 * rather than one array, so this ordered registry is their canonical nav source.
 */
export const EXAMPLE_NAV: Record<string, string> = {
  wallet: "Wallet Sync",
  messaging: "Private Messaging",
  browsing: "Web Browsing",
};
