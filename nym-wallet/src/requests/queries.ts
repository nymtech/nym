import {
  DecCoin,
  GatewayBond,
  InclusionProbabilityResponse,
  MixNodeDetails,
  MixnodeStatusResponse,
  PendingIntervalEvent,
  RewardEstimationResponse,
  StakeSaturationResponse,
  WrappedDelegationEvent,
  NymNodeDetails,
} from '@nymproject/types';
import { Interval, MixnodeSaturationResponse, TGatewayReport, TNodeDescription, TNodeRole } from 'src/types';
import { invokeWrapper } from './wrapper';

export const getAllPendingDelegations = async () =>
  invokeWrapper<WrappedDelegationEvent[]>('get_pending_delegation_events');

export const getMixnodeAvgUptime = async () => invokeWrapper<number | null>('get_mixnode_avg_uptime');

export const getPendingOperatorRewards = async (address: string) =>
  invokeWrapper<DecCoin>('get_pending_operator_rewards', { address });

export const getMixnodeStakeSaturation = async (mixId: number) =>
  invokeWrapper<MixnodeSaturationResponse>('mixnode_stake_saturation', { mixId });

export const getMixnodeRewardEstimation = async (mixId: number) =>
  invokeWrapper<RewardEstimationResponse>('mixnode_reward_estimation', { mixId });

export const getMixnodeStatus = async (mixId: number) =>
  invokeWrapper<MixnodeStatusResponse>('mixnode_status', { mixId });

export const checkMixnodeOwnership = async () => invokeWrapper<boolean>('owns_mixnode');

export const checkGatewayOwnership = async () => invokeWrapper<boolean>('owns_gateway');

export const checkNymNodeOwnership = async () => invokeWrapper<boolean>('owns_nym_node');

export const getInclusionProbability = async (mixId: number) =>
  invokeWrapper<InclusionProbabilityResponse>('mixnode_inclusion_probability', { mixId });

export const getCurrentInterval = async () => invokeWrapper<Interval>('get_current_interval');

export const getNumberOfMixnodeDelegators = async (mixId: number) =>
  invokeWrapper<number>('get_number_of_mixnode_delegators', { mixId });

export const getMixNodeDescription = async (host: string, port: number) =>
  invokeWrapper<TNodeDescription>('get_mix_node_description', { host, port });

export const getNymNodeDescription = async (host: string, port: number) =>
  invokeWrapper<TNodeDescription>('get_nym_node_description', { host, port });

export const getPendingIntervalEvents = async () =>
  invokeWrapper<PendingIntervalEvent[]>('get_pending_interval_events');

export const getGatewayReport = async (identity: string) =>
  invokeWrapper<TGatewayReport>('gateway_report', { identity });

export const computeMixnodeRewardEstimation = async (args: {
  mixId: number;
  performance: string;
  pledgeAmount: number;
  totalDelegation: number;
  profitMarginPercent: string;
  intervalOperatingCost: { denom: 'unym'; amount: string };
}) => invokeWrapper<RewardEstimationResponse>('compute_mixnode_reward_estimation', args);
export const getMixnodeUptime = async (mixId: number) => invokeWrapper<number>('get_mixnode_uptime', { mixId });

export const getNymNodePerformance = async () => invokeWrapper<number>('get_nymnode_performance');

export const getNymNodeUptime = async (nodeId: number) => invokeWrapper<number>('get_nymnode_uptime', { nodeId });

export const getNymNodeStakeSaturation = async (nodeId: number) =>
  invokeWrapper<StakeSaturationResponse>('get_nymnode_stake_saturation', { nodeId });

export const getNymNodeRole = async (nodeId: number) => invokeWrapper<TNodeRole>('get_nymnode_role', { nodeId });
