import type { DelegationWithEverything, WrappedDelegationEvent } from '@nymproject/types';
import { UNBONDED_NODE_IDENTITY_PREFIX } from './delegationIdentity';

/** Synthetic mix_id used in wallet unbonded-delegation tests. */
export const EXAMPLE_UNBONDED_MIX_ID = 1234;

/** Example bonded-node identity (registry miss => wallet uses unbonded prefix instead). */
export const EXAMPLE_HISTORICAL_NODE_IDENTITY = '2ExampleHistoricalNodeIdentityKey00000000000001';

export const EXAMPLE_DELEGATOR_ADDRESS = 'n1exampledelegator00000000000000000000000';

type UnbondedWalletDelegationOptions = {
  mixId?: number;
  amount?: string;
  blockHeight?: bigint;
  owner?: string;
  historicalNodeIdentity?: string | null;
};

export function buildFixedUnbondedWalletDelegation(
  options: UnbondedWalletDelegationOptions = {},
): DelegationWithEverything {
  const mixId = options.mixId ?? EXAMPLE_UNBONDED_MIX_ID;

  return {
    mix_id: mixId,
    node_identity: `${UNBONDED_NODE_IDENTITY_PREFIX}${mixId}`,
    amount: { amount: options.amount ?? '1000000', denom: 'nym' },
    owner: options.owner ?? EXAMPLE_DELEGATOR_ADDRESS,
    block_height: options.blockHeight ?? BigInt(1_000_000),
    delegated_on_iso_datetime: null,
    unclaimed_rewards: null,
    stake_saturation: null,
    avg_uptime_percent: null,
    accumulated_by_delegates: null,
    accumulated_by_operator: null,
    cost_params: null,
    uses_vesting_contract_tokens: false,
    pending_events: [],
    mixnode_is_unbonding: null,
    historical_node_identity: options.historicalNodeIdentity ?? null,
    errors: null,
  };
}

/** Pre-fix wallet backend shape when node registry lookup returned none. */
export function buildLegacyHiddenUnbondedWalletDelegation(
  options: UnbondedWalletDelegationOptions = {},
): DelegationWithEverything {
  return {
    ...buildFixedUnbondedWalletDelegation(options),
    node_identity: '',
    mixnode_is_unbonding: null,
  };
}

export function buildPendingUndelegateEvent(
  nodeIdentity: string,
  mixId: number = EXAMPLE_UNBONDED_MIX_ID,
): WrappedDelegationEvent {
  return {
    node_identity: nodeIdentity,
    event: {
      kind: 'Undelegate',
      mix_id: mixId,
      address: EXAMPLE_DELEGATOR_ADDRESS,
      amount: { amount: '1000000', denom: 'nym' },
      proxy: null,
    },
  };
}

export function buildPendingDelegateEvent(
  nodeIdentity: string,
  mixId: number = EXAMPLE_UNBONDED_MIX_ID,
): WrappedDelegationEvent {
  return {
    node_identity: nodeIdentity,
    event: {
      kind: 'Delegate',
      mix_id: mixId,
      address: EXAMPLE_DELEGATOR_ADDRESS,
      amount: { amount: '1000000', denom: 'nym' },
      proxy: null,
    },
  };
}
