import type { DelegationWithEverything } from '@nymproject/types';
import { formatDelegationNodeIdentityForDisplay } from './delegationIdentity';
import {
  filterVisibleDelegations,
  isUndelegateOnlyDelegation,
  searchDelegations,
  shouldHideDelegationFromList,
} from './delegationListVisibility';
import {
  EXAMPLE_HISTORICAL_NODE_IDENTITY,
  EXAMPLE_UNBONDED_MIX_ID,
  buildFixedUnbondedWalletDelegation,
  buildLegacyHiddenUnbondedWalletDelegation,
} from './unbondedDelegation.fixture';

describe('unbonded delegation wallet visibility acceptance', () => {
  it('hides the pre-fix wallet row with empty node_identity', () => {
    const legacyRow = buildLegacyHiddenUnbondedWalletDelegation();

    expect(legacyRow.mix_id).toBe(EXAMPLE_UNBONDED_MIX_ID);
    expect(legacyRow.node_identity).toBe('');
    expect(shouldHideDelegationFromList(legacyRow)).toBe(true);
    expect(filterVisibleDelegations([legacyRow])).toHaveLength(0);
  });

  it('shows the post-fix wallet row for the same on-chain delegation', () => {
    const fixedRow = buildFixedUnbondedWalletDelegation();

    expect(fixedRow.node_identity).toBe(`unbonded:${EXAMPLE_UNBONDED_MIX_ID}`);
    expect(fixedRow.mixnode_is_unbonding).toBe(true);
    expect(shouldHideDelegationFromList(fixedRow)).toBe(false);
    expect(filterVisibleDelegations([fixedRow])).toHaveLength(1);
    expect(isUndelegateOnlyDelegation(fixedRow)).toBe(true);
  });

  it('does not hide the row when uptime is missing', () => {
    const fixedRow = buildFixedUnbondedWalletDelegation();

    expect(fixedRow.avg_uptime_percent).toBeNull();
    expect(shouldHideDelegationFromList(fixedRow)).toBe(false);
  });

  it('finds the row by mix_id search when historical identity is unavailable', () => {
    const fixedRow = buildFixedUnbondedWalletDelegation();
    const visible = filterVisibleDelegations([fixedRow]) as DelegationWithEverything[];

    expect(searchDelegations(visible, String(EXAMPLE_UNBONDED_MIX_ID))).toHaveLength(1);
    expect(searchDelegations(visible, EXAMPLE_HISTORICAL_NODE_IDENTITY)).toHaveLength(0);
  });

  it('formats undelegate confirmation copy without exposing the synthetic prefix', () => {
    const fixedRow = buildFixedUnbondedWalletDelegation();

    expect(formatDelegationNodeIdentityForDisplay(fixedRow.node_identity, fixedRow.mix_id)).toBe(
      `Node unbonded (mix ${EXAMPLE_UNBONDED_MIX_ID})`,
    );
  });
});
