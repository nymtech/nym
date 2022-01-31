import { invoke } from '@tauri-apps/api'
import {
  Account,
  Balance,
  Coin,
  DelegationResult,
  EnumNodeType,
  Gateway,
  InclusionProbabilityResponse,
  MixNode,
  MixnodeStatusResponse,
  Network,
  Operation,
  RewardEstimationResponse,
  StakeSaturationResponse,
  TauriContractStateParams,
  TauriTxResult,
  TCreateAccount,
  TMixnodeBondDetails,
  TPagedDelegations,
} from '../types'

export const createAccount = async (): Promise<TCreateAccount> => await invoke('create_new_account')

export const signInWithMnemonic = async (mnemonic: string): Promise<Account> =>
  await invoke('connect_with_mnemonic', { mnemonic })

export const signOut = async () => await invoke('logout')

export const minorToMajor = async (amount: string): Promise<Coin> => await invoke('minor_to_major', { amount })

export const majorToMinor = async (amount: string): Promise<Coin> => await invoke('major_to_minor', { amount })

// NOTE: this uses OUTDATED defaults that might have no resemblance with the reality
// as for the actual transaction, the gas cost is being simulated beforehand
export const getGasFee = async (operation: Operation): Promise<Coin> =>
  await invoke('outdated_get_approximate_fee', { operation })

export const delegate = async ({
  type,
  identity,
  amount,
}: {
  type: EnumNodeType
  identity: string
  amount: Coin
}): Promise<DelegationResult> => await invoke(`delegate_to_${type}`, { identity, amount })

export const undelegate = async ({
  type,
  identity,
}: {
  type: EnumNodeType
  identity: string
}): Promise<DelegationResult> => await invoke(`undelegate_from_${type}`, { identity })

export const send = async (args: { amount: Coin; address: string; memo: string }): Promise<TauriTxResult> =>
  await invoke('send', args)

export const checkMixnodeOwnership = async (): Promise<boolean> => await invoke('owns_mixnode')

export const checkGatewayOwnership = async (): Promise<boolean> => await invoke('owns_gateway')

export const bond = async ({
  type,
  data,
  pledge,
  ownerSignature,
}: {
  type: EnumNodeType
  data: MixNode | Gateway
  pledge: Coin
  ownerSignature: string
}): Promise<any> => await invoke(`bond_${type}`, { [type]: data, ownerSignature, pledge })

export const unbond = async (type: EnumNodeType) => await invoke(`unbond_${type}`)

export const userBalance = async (): Promise<Balance> => await invoke('get_balance')

export const getContractParams = async (): Promise<TauriContractStateParams> => await invoke('get_contract_settings')

export const setContractParams = async (params: TauriContractStateParams): Promise<TauriContractStateParams> =>
  await invoke('update_contract_settings', { params })

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

export const updateMixnode = async ({ profitMarginPercent }: { profitMarginPercent: number }) =>
  await invoke('update_mixnode', { profitMarginPercent })

export const getInclusionProbability = async (identity: string): Promise<InclusionProbabilityResponse> =>
  await invoke('mixnode_inclusion_probability', { identity })

export const selectNetwork = async (network: Network): Promise<Account> => await invoke('switch_network', { network })
