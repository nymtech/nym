import { invoke } from '@tauri-apps/api'
import { Account, TCreateAccount } from '../types'

export const createAccount = async (): Promise<TCreateAccount> => await invoke('create_new_account')

export const signInWithMnemonic = async (mnemonic: string): Promise<Account> =>
  await invoke('connect_with_mnemonic', { mnemonic })

export const signOut = async () => await invoke('logout')
