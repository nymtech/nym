import { FeeDetails, DecCoin, Gateway, MixNode, MixNodeCostParams, MixNodeConfigUpdate } from '@nymproject/types';
import { TBondGatewayArgs, TBondMixNodeArgs } from 'src/types';
import { invokeWrapper } from './wrapper';

export const simulateBondGateway = async (args: TBondGatewayArgs) =>
  invokeWrapper<FeeDetails>('simulate_bond_gateway', args);

export const simulateUnbondGateway = async (args: any) => invokeWrapper<FeeDetails>('simulate_unbond_gateway', args);

export const simulateBondMixnode = async (args: TBondMixNodeArgs) =>
  invokeWrapper<FeeDetails>('simulate_bond_mixnode', args);

export const simulateUnbondMixnode = async (args: any) => invokeWrapper<FeeDetails>('simulate_unbond_mixnode', args);

export const simulateUpdateMixnodeCostParams = async (new_costs: MixNodeCostParams) =>
  invokeWrapper<FeeDetails>('simulate_update_mixnode_cost_params', { new_costs });

export const simulateUpdateMixnodeConfig = async (update: MixNodeConfigUpdate) =>
  invokeWrapper<FeeDetails>('simulate_update_mixnode_config', { update });

export const simulateDelegateToMixnode = async (args: { mix_id: number; amount: DecCoin }) =>
  invokeWrapper<FeeDetails>('simulate_delegate_to_mixnode', args);

export const simulateUndelegateFromMixnode = async (mix_id: number) =>
  invokeWrapper<FeeDetails>('simulate_undelegate_from_mixnode', { mix_id });

export const simulateClaimDelegatorReward = async (mix_id: number) =>
  invokeWrapper<FeeDetails>('simulate_claim_delegator_reward', { mix_id });

export const simulateVestingClaimDelegatorReward = async (mix_id: number) =>
  invokeWrapper<FeeDetails>('simulate_vesting_claim_delegator_reward', { mix_id });

export const simulateVestingUndelegateFromMixnode = async (args: any) =>
  invokeWrapper<FeeDetails>('simulate_vesting_undelegate_from_mixnode', args);

export const simulateVestingBondGateway = async (args: { gateway: Gateway; pledge: DecCoin; ownerSignature: string }) =>
  invokeWrapper<FeeDetails>('simulate_vesting_bond_gateway', args);

export const simulateVestingUnbondGateway = async (args: any) =>
  invokeWrapper<FeeDetails>('simulate_vesting_unbond_gateway', args);

export const simulateVestingDelegateToMixnode = async (args: { mix_id: number }) =>
  invokeWrapper<FeeDetails>('simulate_vesting_delegate_to_mixnode', args);

export const simulateVestingBondMixnode = async (args: { mixnode: MixNode; pledge: DecCoin; ownerSignature: string }) =>
  invokeWrapper<FeeDetails>('simulate_vesting_bond_mixnode', args);

export const simulateVestingUnbondMixnode = async () => invokeWrapper<FeeDetails>('simulate_vesting_unbond_mixnode');

export const simulateVestingUpdateMixnodeCostParams = async (new_costs: MixNodeCostParams) =>
  invokeWrapper<FeeDetails>('simulate_vesting_update_mixnode_cost_params', { new_costs });

export const simulateVestingUpdateMixnodeConfig = async (update: MixNodeConfigUpdate) =>
  invokeWrapper<FeeDetails>('simulate_vesting_update_mixnode_config', { update });

export const simulateWithdrawVestedCoins = async (args: any) =>
  invokeWrapper<FeeDetails>('simulate_withdraw_vested_coins', args);

export const simulateSend = async ({ address, amount }: { address: string; amount: DecCoin }) =>
  invokeWrapper<FeeDetails>('simulate_send', { address, amount });

export const simulateClaimOperatorReward = async () => invokeWrapper<FeeDetails>('simulate_claim_operator_reward');

export const simulateVestingClaimOperatorReward = async () =>
  invokeWrapper<FeeDetails>('simulate_vesting_claim_operator_reward');

