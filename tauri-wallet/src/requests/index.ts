import { invoke } from '@tauri-apps/api'
import { TCreateAccount, TSignInWithMnemonic } from '../types'

export const createAccount = async (): Promise<TCreateAccount> =>
  await invoke('create_new_account')

export const signInWithMnemonic = async (
  mnemonic: string
): Promise<TSignInWithMnemonic> =>
  await invoke('connect_with_mnemonic', { mnemonic })
