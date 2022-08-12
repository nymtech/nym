import {
  TNodeType,
  FeeDetails,
  Gateway,
  DecCoin,
  MixNode,
  OriginalVestingResponse,
  Period,
  PledgeData,
  TransactionExecuteResult,
  VestingAccountInfo,
  MixNodeCostParams,
  MixNodeConfigUpdate,
} from '@nymproject/types';
import { Fee } from '@nymproject/types/dist/types/rust/Fee';
import { invokeWrapper } from './wrapper';

export const getLockedCoins = async (): Promise<DecCoin> => invokeWrapper<DecCoin>('locked_coins');

export const getSpendableCoins = async (): Promise<DecCoin> => invokeWrapper<DecCoin>('spendable_coins');

export const getVestingCoins = async (vestingAccountAddress: string): Promise<DecCoin> =>
  invokeWrapper<DecCoin>('vesting_coins', { vestingAccountAddress });

export const getVestedCoins = async (vestingAccountAddress: string): Promise<DecCoin> =>
  invokeWrapper<DecCoin>('vested_coins', { vestingAccountAddress });

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
  pledge: DecCoin;
  ownerSignature: string;
}) => invokeWrapper<TransactionExecuteResult>('vesting_bond_gateway', { gateway, ownerSignature, pledge });

export const vestingUnbondGateway = async (fee?: Fee) =>
  invokeWrapper<TransactionExecuteResult>('vesting_unbond_gateway', { fee });

export const vestingBondMixNode = async ({
  mixnode,
  cost_params,
  pledge,
  ownerSignature,
}: {
  mixnode: MixNode;
  cost_params: MixNodeCostParams;
  pledge: DecCoin;
  ownerSignature: string;
}) => invokeWrapper<TransactionExecuteResult>('vesting_bond_mixnode', { mixnode, cost_params, ownerSignature, pledge });

export const vestingUnbondMixnode = async (fee?: Fee) =>
  invokeWrapper<TransactionExecuteResult>('vesting_unbond_mixnode', { fee });

export const withdrawVestedCoins = async (amount: DecCoin, fee?: Fee) =>
  invokeWrapper<TransactionExecuteResult>('withdraw_vested_coins', { amount, fee });

export const vestingUpdateMixnodeCostParams = async (new_costs: MixNodeCostParams, fee?: Fee) =>
  invokeWrapper<TransactionExecuteResult>('vesting_update_mixnode_cost_params', { new_costs, fee });

export const vestingUpdateMixnodeConfig = async (update: MixNodeConfigUpdate, fee?: Fee) =>
  invokeWrapper<TransactionExecuteResult>('vesting_update_mixnode_config', { update, fee });

export const vestingDelegateToMixnode = async ({
  mix_id,
  amount,
  fee,
}: {
  mix_id: number;
  amount: DecCoin;
  fee?: FeeDetails;
}) => invokeWrapper<TransactionExecuteResult>('vesting_delegate_to_mixnode', { mix_id, amount, fee: fee?.fee });

export const vestingUndelegateFromMixnode = async (mix_id: number) =>
  invokeWrapper<TransactionExecuteResult>('vesting_undelegate_from_mixnode', { mix_id });

export const getVestingAccountInfo = async (address: string) =>
  invokeWrapper<VestingAccountInfo>('get_account_info', { address });

export const getVestingPledgeInfo = async ({
  address,
  type,
}: {
  address?: string;
  type: TNodeType;
}): Promise<PledgeData | undefined> => {
  try {
    return await invokeWrapper<PledgeData>(`vesting_get_${type}_pledge`, { address });
  } catch (e) {
    return undefined;
  }
};

export const vestingDelegatedFree = async (vestingAccountAddress: string) =>
  invokeWrapper<DecCoin>('delegated_free', { vestingAccountAddress });

export const vestingUnbond = async (type: TNodeType) => {
  if (type === 'mixnode') return vestingUnbondMixnode();
  return vestingUnbondGateway();
};

export const vestingClaimOperatorReward = async (fee?: Fee) =>
  invokeWrapper<TransactionExecuteResult>('vesting_claim_operator_reward', { fee });

export const vestingClaimDelegatorRewards = async (mix_id: number) =>
  invokeWrapper<TransactionExecuteResult>('vesting_claim_delegator_reward', { mix_id });
