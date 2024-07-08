import {
  FeeDetails,
  DecCoin,
  Gateway,
  MixNodeCostParams,
  MixNodeConfigUpdate,
  GatewayConfigUpdate,
} from '@nymproject/types';
import { TBondGatewayArgs, TBondMixNodeArgs, TSimulateUpdateBondArgs } from 'src/types';
import { invokeWrapper } from './wrapper';

export const simulateBondGateway = async (args: TBondGatewayArgs) =>
  invokeWrapper<FeeDetails>('simulate_bond_gateway', args);

export const simulateUnbondGateway = async (args: any) => invokeWrapper<FeeDetails>('simulate_unbond_gateway', args);

export const simulateBondMixnode = async (args: TBondMixNodeArgs) =>
  invokeWrapper<FeeDetails>('simulate_bond_mixnode', args);

export const simulateUnbondMixnode = async (args: any) => invokeWrapper<FeeDetails>('simulate_unbond_mixnode', args);

export const simulateUpdateMixnodeCostParams = async (newCosts: MixNodeCostParams) =>
  invokeWrapper<FeeDetails>('simulate_update_mixnode_cost_params', { newCosts });

export const simulateUpdateMixnodeConfig = async (update: MixNodeConfigUpdate) =>
  invokeWrapper<FeeDetails>('simulate_update_mixnode_config', { update });

export const simulateUpdateGatewayConfig = async (update: GatewayConfigUpdate) =>
  invokeWrapper<FeeDetails>('simulate_update_gateway_config', { update });

export const simulateDelegateToMixnode = async (args: { mixId: number; amount: DecCoin }) =>
  invokeWrapper<FeeDetails>('simulate_delegate_to_mixnode', args);

export const simulateUndelegateFromMixnode = async (mixId: number) =>
  invokeWrapper<FeeDetails>('simulate_undelegate_from_mixnode', { mixId });

export const simulateClaimDelegatorReward = async (mixId: number) =>
  invokeWrapper<FeeDetails>('simulate_claim_delegator_reward', { mixId });

export const simulateVestingClaimDelegatorReward = async (mixId: number) =>
  invokeWrapper<FeeDetails>('simulate_vesting_claim_delegator_reward', { mixId });

export const simulateVestingUndelegateFromMixnode = async (args: any) =>
  invokeWrapper<FeeDetails>('simulate_vesting_undelegate_from_mixnode', args);

export const simulateVestingBondGateway = async (args: { gateway: Gateway; pledge: DecCoin; msgSignature: string }) =>
  invokeWrapper<FeeDetails>('simulate_vesting_bond_gateway', args);

export const simulateVestingUnbondGateway = async (args: any) =>
  invokeWrapper<FeeDetails>('simulate_vesting_unbond_gateway', args);

export const simulateVestingDelegateToMixnode = async (args: { mixId: number }) =>
  invokeWrapper<FeeDetails>('simulate_vesting_delegate_to_mixnode', args);

export const simulateVestingBondMixnode = async (args: TBondMixNodeArgs) =>
  invokeWrapper<FeeDetails>('simulate_vesting_bond_mixnode', args);

export const simulateVestingUnbondMixnode = async () => invokeWrapper<FeeDetails>('simulate_vesting_unbond_mixnode');

export const simulateVestingUpdateMixnodeCostParams = async (newCosts: MixNodeCostParams) =>
  invokeWrapper<FeeDetails>('simulate_vesting_update_mixnode_cost_params', { newCosts });

export const simulateVestingUpdateMixnodeConfig = async (update: MixNodeConfigUpdate) =>
  invokeWrapper<FeeDetails>('simulate_vesting_update_mixnode_config', { update });

export const simulateVestingUpdateGatewayConfig = async (update: GatewayConfigUpdate) =>
  invokeWrapper<FeeDetails>('simulate_vesting_update_gateway_config', { update });

export const simulateWithdrawVestedCoins = async (args: any) =>
  invokeWrapper<FeeDetails>('simulate_withdraw_vested_coins', args);

export const simulateSend = async ({ address, amount, memo }: { address: string; amount: DecCoin; memo: string }) =>
  invokeWrapper<FeeDetails>('simulate_send', { address, amount, memo });

export const getCustomFees = async ({ feesAmount }: { feesAmount: DecCoin }) =>
  invokeWrapper<FeeDetails>('get_custom_fees', { feesAmount });

export const simulateClaimOperatorReward = async () => invokeWrapper<FeeDetails>('simulate_claim_operator_reward');

export const simulateVestingClaimOperatorReward = async () =>
  invokeWrapper<FeeDetails>('simulate_vesting_claim_operator_reward');

export const simulateUpdateBond = async (args: TSimulateUpdateBondArgs) =>
  invokeWrapper<FeeDetails>('simulate_update_pledge', args);

export const simulateVestingUpdateBond = async (args: TSimulateUpdateBondArgs) =>
  invokeWrapper<FeeDetails>('simulate_vesting_update_pledge', args);
