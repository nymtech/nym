import { invoke } from '@tauri-apps/api';
import { DelegationEvent } from 'src/types/rust/delegationevent';
import {
  Balance,
  Coin,
  InclusionProbabilityResponse,
  MixnodeStatusResponse,
  Operation,
 // RewardEstimationResponse,
  StakeSaturationResponse,
  TMixnodeBondDetails,
  TPagedDelegations,
  Epoch,
} from '../types';

export const getReverseMixDelegations = async (): Promise<TPagedDelegations> => {
  const res: TPagedDelegations = await invoke('get_reverse_mix_delegations_paged');
  return res;
};

export const getReverseGatewayDelegations = async (): Promise<TPagedDelegations> => {
  const res: TPagedDelegations = await invoke('get_reverse_gateway_delegations_paged');
  return res;
};

export const getPendingDelegations = async (): Promise<DelegationEvent[]> => {
  const res: DelegationEvent[] = await invoke('get_pending_delegation_events');
  return res;
};

export const getMixnodeBondDetails = async (): Promise<TMixnodeBondDetails | null> => {
  const res: TMixnodeBondDetails = await invoke('mixnode_bond_details');
  return res;
};

export const getMixnodeStakeSaturation = async (identity: string): Promise<StakeSaturationResponse> => {
  const res: StakeSaturationResponse = await invoke('mixnode_stake_saturation', { identity });
  return res;
};

export const getMixnodeStatus = async (identity: string): Promise<MixnodeStatusResponse> => {
  const res: MixnodeStatusResponse = await invoke('mixnode_status', { identity });
  return res;
};

export const checkMixnodeOwnership = async (): Promise<boolean> => {
  const res: boolean = await invoke('owns_mixnode');
  return res;
};

export const checkGatewayOwnership = async (): Promise<boolean> => {
  const res: boolean = await invoke('owns_gateway');
  return res;
};

// NOTE: this uses OUTDATED defaults that might have no resemblance with the reality
// as for the actual transaction, the gas cost is being simulated beforehand
export const getGasFee = async (operation: Operation): Promise<Coin> => {
  const res: Coin = await invoke('outdated_get_approximate_fee', { operation });
  return res;
};

export const getInclusionProbability = async (identity: string): Promise<InclusionProbabilityResponse> => {
  const res: InclusionProbabilityResponse = await invoke('mixnode_inclusion_probability', { identity });
  return res;
};

export const userBalance = async (): Promise<Balance> => {
  const res: Balance = await invoke('get_balance');
  return res;
};

export const getCurrentEpoch = async (): Promise<Epoch> => {
  const res: Epoch = await invoke('get_current_epoch');
  return res;
};
