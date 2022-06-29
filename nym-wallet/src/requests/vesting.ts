import {
  EnumNodeType,
  FeeDetails,
  Gateway,
  MajorCurrencyAmount,
  MixNode,
  OriginalVestingResponse,
  Period,
  PledgeData,
  TransactionExecuteResult,
  VestingAccountInfo,
} from '@nymproject/types';
import { invokeWrapper } from './wrapper';
import { TBondGatewayArgs, TBondMixNodeArgs } from '../types';

export const getLockedCoins = async (): Promise<MajorCurrencyAmount> =>
  invokeWrapper<MajorCurrencyAmount>('locked_coins');

export const getSpendableCoins = async (): Promise<MajorCurrencyAmount> =>
  invokeWrapper<MajorCurrencyAmount>('spendable_coins');

export const getVestingCoins = async (vestingAccountAddress: string): Promise<MajorCurrencyAmount> =>
  invokeWrapper<MajorCurrencyAmount>('vesting_coins', { vestingAccountAddress });

export const getVestedCoins = async (vestingAccountAddress: string): Promise<MajorCurrencyAmount> =>
  invokeWrapper<MajorCurrencyAmount>('vested_coins', { vestingAccountAddress });

export const getOriginalVesting = async (vestingAccountAddress: string): Promise<OriginalVestingResponse> => {
  const res = await invokeWrapper<OriginalVestingResponse>('original_vesting', { vestingAccountAddress });
  return { ...res, amount: res.amount };
};

export const getCurrentVestingPeriod = async (address: string) =>
  invokeWrapper<Period>('get_current_vesting_period', { address });

export const vestingBondGateway = async ({
  gateway,
  pledge,
  ownerSignature,
}: {
  gateway: Gateway;
  pledge: MajorCurrencyAmount;
  ownerSignature: string;
}) => invokeWrapper<TransactionExecuteResult>('vesting_bond_gateway', { gateway, ownerSignature, pledge });

export const vestingUnbondGateway = async () => invokeWrapper<TransactionExecuteResult>('vesting_unbond_gateway');

export const vestingBondMixNode = async ({
  mixnode,
  pledge,
  ownerSignature,
}: {
  mixnode: MixNode;
  pledge: MajorCurrencyAmount;
  ownerSignature: string;
}) => invokeWrapper<TransactionExecuteResult>('vesting_bond_mixnode', { mixnode, ownerSignature, pledge });

export const vestingUnbondMixnode = async () => invokeWrapper<TransactionExecuteResult>('vesting_unbond_mixnode');

export const withdrawVestedCoins = async (amount: MajorCurrencyAmount) =>
  invokeWrapper<TransactionExecuteResult>('withdraw_vested_coins', { amount });

export const vestingUpdateMixnode = async (profitMarginPercent: number) =>
  invokeWrapper<TransactionExecuteResult>('vesting_update_mixnode', { profitMarginPercent });

export const vestingDelegateToMixnode = async ({
  identity,
  amount,
  fee,
}: {
  identity: string;
  amount: MajorCurrencyAmount;
  fee?: FeeDetails;
}) => invokeWrapper<TransactionExecuteResult>('vesting_delegate_to_mixnode', { identity, amount, fee: fee?.fee });

export const vestingUndelegateFromMixnode = async (identity: string) =>
  invokeWrapper<TransactionExecuteResult>('vesting_undelegate_from_mixnode', { identity });

export const getVestingAccountInfo = async (address: string) =>
  invokeWrapper<VestingAccountInfo>('get_account_info', { address });

export const getVestingPledgeInfo = async ({
  address,
  type,
}: {
  address?: string;
  type: EnumNodeType;
}): Promise<PledgeData | undefined> => {
  try {
    return await invokeWrapper<PledgeData>(`vesting_get_${type}_pledge`, { address });
  } catch (e) {
    return undefined;
  }
};

export const vestingDelegatedFree = async (vestingAccountAddress: string) =>
  invokeWrapper<MajorCurrencyAmount>('delegated_free', { vestingAccountAddress });

export const vestingUnbond = async (type: EnumNodeType) => {
  if (type === EnumNodeType.mixnode) return vestingUnbondMixnode();
  return vestingUnbondGateway();
};

export const vestingBond = async (args: { type: EnumNodeType } & (TBondMixNodeArgs | TBondGatewayArgs)) => {
  const { type, ...other } = args;
  if (type === EnumNodeType.mixnode) return vestingBondMixNode(other as TBondMixNodeArgs);
  return vestingBondGateway(other as TBondGatewayArgs);
};

export const vestingClaimOperatorRewards = async () =>
  invokeWrapper<TransactionExecuteResult>('vesting_claim_operator_reward');

export const vestingCompoundOperatorRewards = async () =>
  invokeWrapper<TransactionExecuteResult>('vesting_compound_operator_reward');

export const vestingClaimDelegatorRewards = async (mixIdentity: string) =>
  invokeWrapper<TransactionExecuteResult>('vesting_claim_delegator_reward', { mixIdentity });

export const vestingCompoundDelegatorRewards = async (mixIdentity: string) =>
  invokeWrapper<TransactionExecuteResult>('vesting_compound_delegator_reward', { mixIdentity });
