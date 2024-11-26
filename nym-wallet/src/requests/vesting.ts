import {
  TNodeType,
  Gateway,
  DecCoin,
  MixNode,
  OriginalVestingResponse,
  Period,
  PledgeData,
  TransactionExecuteResult,
  VestingAccountInfo,
  NodeCostParams,
  MixNodeConfigUpdate,
  GatewayConfigUpdate,
} from '@nymproject/types';
import { Fee } from '@nymproject/types/dist/types/rust/Fee';
import { invokeWrapper } from './wrapper';
import { TBondGatewaySignatureArgs, TBondMixnodeSignatureArgs, TUpdateBondArgs } from '../types';

export const getLockedCoins = async (): Promise<DecCoin> => invokeWrapper<DecCoin>('locked_coins');

export const getSpendableCoins = async (): Promise<DecCoin> => invokeWrapper<DecCoin>('spendable_coins');

export const getSpendableVestedCoins = async (): Promise<DecCoin> => invokeWrapper<DecCoin>('spendable_vested_coins');

export const getSpendableRewardCoins = async (): Promise<DecCoin> => invokeWrapper<DecCoin>('spendable_reward_coins');

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
  msgSignature,
}: {
  gateway: Gateway;
  pledge: DecCoin;
  msgSignature: string;
}) => invokeWrapper<TransactionExecuteResult>('vesting_bond_gateway', { gateway, msgSignature, pledge });

export const vestingGenerateGatewayMsgPayload = async (args: Omit<TBondGatewaySignatureArgs, 'tokenPool'>) =>
  invokeWrapper<string>('vesting_generate_gateway_bonding_msg_payload', args);

export const vestingUnbondGateway = async (fee?: Fee) =>
  invokeWrapper<TransactionExecuteResult>('vesting_unbond_gateway', { fee });

export const vestingBondMixNode = async ({
  mixnode,
  costParams,
  pledge,
  msgSignature,
}: {
  mixnode: MixNode;
  costParams: NodeCostParams;
  pledge: DecCoin;
  msgSignature: string;
}) => invokeWrapper<TransactionExecuteResult>('vesting_bond_mixnode', { mixnode, costParams, msgSignature, pledge });

export const vestingGenerateMixnodeMsgPayload = async (args: Omit<TBondMixnodeSignatureArgs, 'tokenPool'>) =>
  invokeWrapper<string>('vesting_generate_mixnode_bonding_msg_payload', args);

export const vestingUnbondMixnode = async (fee?: Fee) =>
  invokeWrapper<TransactionExecuteResult>('vesting_unbond_mixnode', { fee });

export const withdrawVestedCoins = async (amount: DecCoin, fee?: Fee) =>
  invokeWrapper<TransactionExecuteResult>('withdraw_vested_coins', { amount, fee });

export const vestingUpdateNodeCostParams = async (newCosts: NodeCostParams, fee?: Fee) =>
  invokeWrapper<TransactionExecuteResult>('vesting_update_mixnode_cost_params', { newCosts, fee });

export const vestingUpdateMixnodeConfig = async (update: MixNodeConfigUpdate, fee?: Fee) =>
  invokeWrapper<TransactionExecuteResult>('vesting_update_mixnode_config', { update, fee });

export const vestingUpdateGatewayConfig = async (update: GatewayConfigUpdate, fee?: Fee) =>
  invokeWrapper<TransactionExecuteResult>('vesting_update_gateway_config', { update, fee });

export const vestingDelegateToMixnode = async (mixId: number, amount: DecCoin, fee?: Fee) =>
  invokeWrapper<TransactionExecuteResult>('vesting_delegate_to_mixnode', { mixId, amount, fee });

export const vestingUndelegateFromMixnode = async (mixId: number) =>
  invokeWrapper<TransactionExecuteResult>('vesting_undelegate_from_mixnode', { mixId });

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

export const vestingClaimDelegatorRewards = async (mixId: number) =>
  invokeWrapper<TransactionExecuteResult>('vesting_claim_delegator_reward', { mixId });

export const vestingUpdateBond = async (args: TUpdateBondArgs) =>
  invokeWrapper<TransactionExecuteResult>('vesting_update_pledge', args);
