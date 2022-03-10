import { invoke } from '@tauri-apps/api';
import { VestingAccountInfo } from 'src/types/rust/vestingaccountinfo';
import { majorToMinor, minorToMajor } from '.';
import { Coin, DelegationResult, OriginalVestingResponse, Period } from '../types';

export const getLockedCoins = async (address: string): Promise<Coin> => {
  const res: Coin = await invoke('locked_coins', { address });
  return minorToMajor(res.amount);
};
export const getSpendableCoins = async (vestingAccountAddress?: string): Promise<Coin> => {
  const res: Coin = await invoke('spendable_coins', { vestingAccountAddress });
  return minorToMajor(res.amount);
};

export const getVestingCoins = async (vestingAccountAddress: string): Promise<Coin> => {
  const res: Coin = await invoke('vesting_coins', { vestingAccountAddress });
  return minorToMajor(res.amount);
};

export const getVestedCoins = async (vestingAccountAddress: string): Promise<Coin> => {
  const res: Coin = await invoke('vested_coins', { vestingAccountAddress });
  return minorToMajor(res.amount);
};

export const getOriginalVesting = async (vestingAccountAddress: string): Promise<OriginalVestingResponse> => {
  const res: OriginalVestingResponse = await invoke('original_vesting', { vestingAccountAddress });
  const majorValue = await minorToMajor(res.amount.amount);
  return { ...res, amount: majorValue };
};

export const withdrawVestedCoins = async (amount: string) => {
  const minor = await majorToMinor(amount);
  await invoke('withdraw_vested_coins', { amount: { amount: minor.amount, denom: 'Minor' } });
};

export const getCurrentVestingPeriod = async (address: string): Promise<Period> =>
  invoke('get_current_vesting_period', { address });

export const vestingDelegateToMixnode = async ({
  identity,
  amount,
}: {
  identity: string;
  amount: Coin;
}): Promise<DelegationResult> => invoke('vesting_delegate_to_mixnode', { identity, amount });

export const vestingUnelegateFromMixnode = async (identity: string): Promise<DelegationResult> =>
  invoke('vesting_delegate_to_mixnode', { identity });

export const getVestingAccountInfo = async (address: string): Promise<VestingAccountInfo> =>
  invoke('get_account_info', { address });
