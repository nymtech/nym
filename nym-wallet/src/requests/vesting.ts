import { invoke } from '@tauri-apps/api'
import { majorToMinor, minorToMajor } from '.'
import { Coin } from '../types'

export const getLockedCoins = async (address: string): Promise<Coin> => {
  const res: Coin = await invoke('locked_coins', { address })
  return await minorToMajor(res.amount)
}
export const getSpendableCoins = async (vestingAccountAddress?: string): Promise<Coin> => {
  const res: Coin = await invoke('spendable_coins', { vestingAccountAddress })
  return await minorToMajor(res.amount)
}

export const getVestingCoins = async (vestingAccountAddress: string): Promise<Coin> => {
  const res: Coin = await invoke('vesting_coins', { vestingAccountAddress })
  return await minorToMajor(res.amount)
}

export const getVestedCoins = async (vestingAccountAddress: string): Promise<Coin> => {
  const res: Coin = await invoke('vested_coins', { vestingAccountAddress })
  return await minorToMajor(res.amount)
}

export const originalVesting = async (vestingAccountAddress: string): Promise<Coin> => {
  const res: Coin = await invoke('original_vesting', { vestingAccountAddress })
  return await minorToMajor(res.amount)
}

export const withdrawVestedCoins = async (amount: string) => {
  const minor = await majorToMinor(amount)
  await invoke('withdraw_vested_coins', { amount: { amount: minor.amount, denom: 'Minor' } })
}
