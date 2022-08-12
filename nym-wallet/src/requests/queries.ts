import {
  TPagedDelegations,
  DelegationEvent,
  StakeSaturationResponse,
  MixnodeStatusResponse,
  InclusionProbabilityResponse,
  DecCoin,
  MixNodeDetails,
  GatewayBond,
} from '@nymproject/types';
import { Interval, TNodeDescription } from 'src/types';
import { invokeWrapper } from './wrapper';

export const getAllPendingDelegations = async () => invokeWrapper<DelegationEvent[]>('get_pending_delegation_events');

export const getMixnodeBondDetails = async () => invokeWrapper<MixNodeDetails | null>('mixnode_bond_details');
export const getGatewayBondDetails = async () => invokeWrapper<GatewayBond | null>('gateway_bond_details');

export const getPendingOperatorRewards = async (address: string) =>
  invokeWrapper<DecCoin>('get_pending_operator_rewards', { address });

export const getMixnodeStakeSaturation = async (mix_id: number) =>
  invokeWrapper<StakeSaturationResponse>('mixnode_stake_saturation', { mix_id });

// export const getMixnodeRewardEstimation = async (mix_id: number) =>
//   invokeWrapper<RewardEstimationResponse>('mixnode_reward_estimation', { identity });

export const getMixnodeStatus = async (mix_id: number) =>
  invokeWrapper<MixnodeStatusResponse>('mixnode_status', { mix_id });

export const checkMixnodeOwnership = async () => invokeWrapper<boolean>('owns_mixnode');

export const checkGatewayOwnership = async () => invokeWrapper<boolean>('owns_gateway');

export const getInclusionProbability = async (mix_id: number) =>
  invokeWrapper<InclusionProbabilityResponse>('mixnode_inclusion_probability', { mix_id });

export const getCurrentInterval = async () => invokeWrapper<Interval>('get_current_interval');

export const getNumberOfMixnodeDelegators = async (mix_id: number) =>
  invokeWrapper<number>('get_number_of_mixnode_delegators', { mix_id });

export const getNodeDescription = async (host: string, port: number) =>
  invokeWrapper<TNodeDescription>('get_mix_node_description', { host, port });
