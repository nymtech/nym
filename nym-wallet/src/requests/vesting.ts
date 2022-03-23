import { invoke } from '@tauri-apps/api';
import { VestingAccountInfo } from 'src/types/rust/vestingaccountinfo';
import { majorToMinor, minorToMajor } from './coin';
import {
  Coin,
  DelegationResult,
  EnumNodeType,
  Gateway,
  MixNode,
  OriginalVestingResponse,
  Period,
  PledgeData,
} from '../types';

export const getLockedCoins = async (): Promise<Coin> => {
  const coin: Coin = await invoke('locked_coins');
  const major = await minorToMajor(coin.amount);
  return major;
};

export const getSpendableCoins = async (): Promise<Coin> => {
  const coin: Coin = await invoke('spendable_coins');
  const major = await minorToMajor(coin.amount);
  return major;
};

export const getVestingCoins = async (vestingAccountAddress: string): Promise<Coin> => {
  const coin: Coin = await invoke('vesting_coins', { vestingAccountAddress });
  const major = await minorToMajor(coin.amount);
  return major;
};

export const getVestedCoins = async (vestingAccountAddress: string): Promise<Coin> => {
  const coin: Coin = await invoke('vested_coins', { vestingAccountAddress });
  const major = await minorToMajor(coin.amount);
  return major;
};

export const getOriginalVesting = async (vestingAccountAddress: string): Promise<OriginalVestingResponse> => {
  const res: OriginalVestingResponse = await invoke('original_vesting', { vestingAccountAddress });
  const major = await minorToMajor(res.amount.amount);
  return { ...res, amount: major };
};

export const withdrawVestedCoins = async (amount: string): Promise<void> => {
  const minor = await majorToMinor(amount);
  await invoke('withdraw_vested_coins', { amount: { amount: minor.amount, denom: 'Minor' } });
};

export const getCurrentVestingPeriod = async (address: string): Promise<Period> => {
  const res: Period = await invoke('get_current_vesting_period', { address });
  return res;
};

export const vestingBond = async ({
  type,
  data,
  pledge,
  ownerSignature,
}: {
  type: EnumNodeType;
  data: MixNode | Gateway;
  pledge: Coin;
  ownerSignature: string;
}): Promise<void> => {
  await invoke(`vesting_bond_${type}`, { [type]: data, ownerSignature, pledge });
};

export const vestingUnbond = async (type: EnumNodeType): Promise<void> => {
  await invoke(`vesting_unbond_${type}`);
};

export const vestingDelegateToMixnode = async ({
  identity,
  amount,
}: {
  identity: string;
  amount: Coin;
}): Promise<DelegationResult> => {
  const res: DelegationResult = await invoke('vesting_delegate_to_mixnode', { identity, amount });
  return res;
};

export const vestingUnelegateFromMixnode = async (identity: string): Promise<DelegationResult> => {
  const res: DelegationResult = await invoke('vesting_undelegate_from_mixnode', { identity });
  return res;
};

export const getVestingAccountInfo = async (address: string): Promise<VestingAccountInfo> => {
  const res: VestingAccountInfo = await invoke('get_account_info', { address });
  return res;
};

export const getVestingPledgeInfo = async ({
  address,
  type,
}: {
  address?: string;
  type: EnumNodeType;
}): Promise<PledgeData | undefined> => {
  try {
    const res: PledgeData = await invoke(`vesting_get_${type}_pledge`, { address });
    return res;
  } catch (e) {
    return undefined;
  }
};

export const vestingUpdateMixnode = async (profitMarginPercent: number): Promise<void> => {
  await invoke('vesting_update_mixnode', { profitMarginPercent });
};

export const vestingDelegatedFree = async (vestingAccountAddress: string): Promise<Coin> => {
  const res: Coin = await invoke('delegated_free', { vestingAccountAddress });
  return res;
};
