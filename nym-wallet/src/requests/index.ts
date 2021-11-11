import { invoke } from '@tauri-apps/api'
import {
  Balance,
  Coin,
  DelegationResult,
  EnumNodeType,
  Gateway,
  MixNode,
  Operation,
  TauriStateParams,
  TauriTxResult,
  TCreateAccount,
  TDelegation,
  TSignInWithMnemonic,
} from '../types'

export const createAccount = async (): Promise<TCreateAccount> =>
  await invoke('create_new_account')

export const signInWithMnemonic = async (
  mnemonic: string
): Promise<TSignInWithMnemonic> =>
  await invoke('connect_with_mnemonic', { mnemonic })

export const minorToMajor = async (amount: string): Promise<Coin> =>
  await invoke('minor_to_major', { amount })

export const majorToMinor = async (amount: string): Promise<Coin> =>
  await invoke('major_to_minor', { amount })

export const getGasFee = async (operation: Operation): Promise<Coin> =>
  await invoke('get_fee', { operation })

export const delegate = async ({
  type,
  identity,
  amount,
}: {
  type: EnumNodeType
  identity: string
  amount: Coin
}): Promise<DelegationResult> =>
  await invoke(`delegate_to_${type}`, { identity, amount })

export const undelegate = async ({
  type,
  identity,
}: {
  type: EnumNodeType
  identity: string
}): Promise<DelegationResult> =>
  await invoke(`undelegate_from_${type}`, { identity })

export const send = async (args: {
  amount: Coin
  address: string
  memo: string
}): Promise<TauriTxResult> => await invoke('send', args)
export const checkMixnodeOwnership = async (): Promise<boolean> =>
  await invoke('owns_mixnode')

export const checkGatewayOwnership = async (): Promise<boolean> =>
  await invoke('owns_gateway')

export const bond = async ({
  type,
  data,
  amount,
}: {
  type: EnumNodeType
  data: MixNode | Gateway
  amount: Coin
}): Promise<any> => await invoke(`bond_${type}`, { [type]: data, bond: amount })

export const unbond = async (type: EnumNodeType) =>
  await invoke(`unbond_${type}`)

export const getBalance = async (): Promise<Balance> =>
  await invoke('get_balance')

export const getContractParams = async (): Promise<TauriStateParams> =>
  await invoke('get_state_params')

export const setContractParams = async (
  params: TauriStateParams
): Promise<TauriStateParams> => await invoke('update_state_params', { params })

export const getReverseMixDelegations = async (): Promise<TDelegation> =>
  await invoke('get_reverse_mix_delegations_paged')

export const getReverseGatewayDelegations = async (): Promise<TDelegation> =>
  await invoke('get_reverse_gateway_delegations_paged')
