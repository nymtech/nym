import { invoke } from '@tauri-apps/api'
import {
  Coin,
  DelegationResult,
  EnumNodeType,
  Operation,
  TauriTxResult,
  TCreateAccount,
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
