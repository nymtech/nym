import { invoke } from '@tauri-apps/api'
import { Coin } from '../types'

export const getLockedCoins = async (address: string): Promise<Coin> => await invoke('locked_coins', { address })

export const getSpendableCoins = async (vestingAccountAddress?: string): Promise<Coin> =>
  await invoke('spendable_coins', { vestingAccountAddress })

export const getVestingCoins = async (vestingAccountAddress: string): Promise<Coin> =>
  await invoke('vesting_coins', { vestingAccountAddress })

export const getVestedCoins = async (vestingAccountAddress: string): Promise<Coin> =>
  await invoke('vested_coins', { vestingAccountAddress })
