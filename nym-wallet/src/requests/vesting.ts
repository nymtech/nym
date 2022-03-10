import { invoke } from '@tauri-apps/api'
import { VestingAccountInfo } from 'src/types/rust/vestingaccountinfo'
import { majorToMinor, minorToMajor } from '.'
import {
  Coin,
  DelegationResult,
  EnumNodeType,
  Gateway,
  MixNode,
  OriginalVestingResponse,
  Period,
  PledgeData,
} from '../types'

export const getLockedCoins = async (): Promise<Coin> => {
  const res: Coin = await invoke('locked_coins')
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

export const getOriginalVesting = async (vestingAccountAddress: string): Promise<OriginalVestingResponse> => {
  const res: OriginalVestingResponse = await invoke('original_vesting', { vestingAccountAddress })
  const majorValue = await minorToMajor(res.amount.amount)
  return { ...res, amount: majorValue }
}

export const withdrawVestedCoins = async (amount: string) => {
  const minor = await majorToMinor(amount)
  await invoke('withdraw_vested_coins', { amount: { amount: minor.amount, denom: 'Minor' } })
}

export const getCurrentVestingPeriod = async (address: string): Promise<Period> =>
  await invoke('get_current_vesting_period', { address })

export const vestingBond = async ({
  type,
  data,
  pledge,
  ownerSignature,
}: {
  type: EnumNodeType
  data: MixNode | Gateway
  pledge: Coin
  ownerSignature: string
}): Promise<any> => await invoke(`vesting_bond_${type}`, { [type]: data, ownerSignature, pledge })

export const vestingUnbond = async (type: EnumNodeType) => await invoke(`vesting_unbond_${type}`)

export const vestingDelegateToMixnode = async ({
  identity,
  amount,
}: {
  identity: string
  amount: Coin
}): Promise<DelegationResult> => await invoke('vesting_delegate_to_mixnode', { identity, amount })

export const vestingUnelegateFromMixnode = async (identity: string): Promise<DelegationResult> =>
  await invoke('vesting_undelegate_from_mixnode', { identity })

export const getVestingAccountInfo = async (address: string): Promise<VestingAccountInfo> =>
  await invoke('get_account_info', { address })

export const getVestingPledgeInfo = async ({
  address,
  type,
}: {
  address?: string
  type: EnumNodeType
}): Promise<PledgeData | undefined> => {
  try {
    return await invoke(`vesting_get_${type}_pledge`, { address })
  } catch (e) {
    return undefined
  }
}

export const vestingUpdateMixnode = async (profitMarginPercent: number) =>
  await invoke('vesting_update_mixnode', { profitMarginPercent })

export const vestingDelegatedFree = async (vestingAccountAddress: string) => {
  await invoke('delegated_free', { vestingAccountAddress })
}
