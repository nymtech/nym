import {
  TPagedDelegations,
  DelegationEvent,
  StakeSaturationResponse,
  MixnodeStatusResponse,
  InclusionProbabilityResponse,
  DecCoin,
  MixNodeBond,
  GatewayBond,
} from '@nymproject/types';
import { Epoch } from 'src/types';
import { invokeWrapper } from './wrapper';

export const getReverseMixDelegations = async () =>
  invokeWrapper<TPagedDelegations>('get_reverse_mix_delegations_paged');

export const getReverseGatewayDelegations = async () =>
  invokeWrapper<TPagedDelegations>('get_reverse_gateway_delegations_paged');

export const getPendingDelegations = async () => invokeWrapper<DelegationEvent[]>('get_pending_delegation_events');

export const getPendingVestingDelegations = async () =>
  invokeWrapper<DelegationEvent[]>('get_pending_vesting_delegation_events');

export const getAllPendingDelegations = async () =>
  invokeWrapper<DelegationEvent[]>('get_all_pending_delegation_events');

export const getMixnodeBondDetails = async () => invokeWrapper<MixNodeBond | null>('mixnode_bond_details');
export const getGatewayBondDetails = async () => invokeWrapper<GatewayBond | null>('gateway_bond_details');

export const getOperatorRewards = async (address: string) =>
  invokeWrapper<DecCoin>('get_operator_rewards', { address });

export const getMixnodeStakeSaturation = async (identity: string) =>
  invokeWrapper<StakeSaturationResponse>('mixnode_stake_saturation', { identity });

// export const getMixnodeRewardEstimation = async (identity: string) =>
//   invokeWrapper<RewardEstimationResponse>('mixnode_reward_estimation', { identity });

export const getMixnodeStatus = async (identity: string) =>
  invokeWrapper<MixnodeStatusResponse>('mixnode_status', { identity });

export const checkMixnodeOwnership = async () => invokeWrapper<boolean>('owns_mixnode');

export const checkGatewayOwnership = async () => invokeWrapper<boolean>('owns_gateway');

export const getInclusionProbability = async (identity: string) =>
  invokeWrapper<InclusionProbabilityResponse>('mixnode_inclusion_probability', { identity });

export const getCurrentEpoch = async () => invokeWrapper<Epoch>('get_current_epoch');
