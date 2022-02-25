import { invoke } from '@tauri-apps/api'
import {
  Balance,
  Coin,
  InclusionProbabilityResponse,
  MixnodeStatusResponse,
  Operation,
  RewardEstimationResponse,
  StakeSaturationResponse,
  TMixnodeBondDetails,
  TPagedDelegations,
} from '../types'

export const getReverseMixDelegations = async (): Promise<TPagedDelegations> =>
  await invoke('get_reverse_mix_delegations_paged')

export const getReverseGatewayDelegations = async (): Promise<TPagedDelegations> =>
  await invoke('get_reverse_gateway_delegations_paged')

export const getMixnodeBondDetails = async (): Promise<TMixnodeBondDetails | null> =>
  await invoke('mixnode_bond_details')

export const getMixnodeStakeSaturation = async (identity: string): Promise<StakeSaturationResponse> =>
  await invoke('mixnode_stake_saturation', { identity })

export const getMixnodeRewardEstimation = async (identity: string): Promise<RewardEstimationResponse> =>
  await invoke('mixnode_reward_estimation', { identity })

export const getMixnodeStatus = async (identity: string): Promise<MixnodeStatusResponse> =>
  await invoke('mixnode_status', { identity })

export const checkMixnodeOwnership = async (): Promise<boolean> => await invoke('owns_mixnode')

export const checkGatewayOwnership = async (): Promise<boolean> => await invoke('owns_gateway')

// NOTE: this uses OUTDATED defaults that might have no resemblance with the reality
// as for the actual transaction, the gas cost is being simulated beforehand
export const getGasFee = async (operation: Operation): Promise<Coin> =>
  await invoke('outdated_get_approximate_fee', { operation })

export const getInclusionProbability = async (identity: string): Promise<InclusionProbabilityResponse> =>
  await invoke('mixnode_inclusion_probability', { identity })

export const userBalance = async (): Promise<Balance> => await invoke('get_balance')
