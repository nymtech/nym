import {
  StakeSaturationResponse,
  MixnodeStatusResponse,
  InclusionProbabilityResponse,
  DecCoin,
  MixNodeDetails,
  GatewayBond,
  WrappedDelegationEvent,
} from '@nymproject/types';
import { Interval, TNodeDescription } from 'src/types';
import { invokeWrapper } from './wrapper';

export const getAllPendingDelegations = async () =>
  invokeWrapper<WrappedDelegationEvent[]>('get_pending_delegation_events');

export const getMixnodeBondDetails = async () => invokeWrapper<MixNodeDetails | null>('mixnode_bond_details');
export const getGatewayBondDetails = async () => invokeWrapper<GatewayBond | null>('gateway_bond_details');

export const getPendingOperatorRewards = async (address: string) =>
  invokeWrapper<DecCoin>('get_pending_operator_rewards', { address });

export const getMixnodeStakeSaturation = async (mixId: number) =>
  invokeWrapper<StakeSaturationResponse>('mixnode_stake_saturation', { mixId });

// export const getMixnodeRewardEstimation = async (mixId: number) =>
//   invokeWrapper<RewardEstimationResponse>('mixnode_reward_estimation', { identity });

export const getMixnodeStatus = async (mixId: number) =>
  invokeWrapper<MixnodeStatusResponse>('mixnode_status', { mixId });

export const checkMixnodeOwnership = async () => invokeWrapper<boolean>('owns_mixnode');

export const checkGatewayOwnership = async () => invokeWrapper<boolean>('owns_gateway');

export const getInclusionProbability = async (mixId: number) =>
  invokeWrapper<InclusionProbabilityResponse>('mixnode_inclusion_probability', { mixId });

export const getCurrentInterval = async () => invokeWrapper<Interval>('get_current_interval');

export const getNumberOfMixnodeDelegators = async (mixId: number) =>
  invokeWrapper<number>('get_number_of_mixnode_delegators', { mixId });

export const getNodeDescription = async (host: string, port: number) =>
  invokeWrapper<TNodeDescription>('get_mix_node_description', { host, port });

export const computeMixnodeRewardEstimation = async (args: {
  identity: string;
  uptime: number;
  isActive: boolean;
  pledgeAmount: number;
  totalDelegation: number;
}) => invokeWrapper<any>('compute_mixnode_reward_estimation', args);
