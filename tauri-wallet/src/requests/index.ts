import { invoke } from '@tauri-apps/api'
import { Coin, Operation, TCreateAccount, TSignInWithMnemonic } from '../types'

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
