import { FeeDetails, MajorCurrencyAmount } from '@nymproject/types';
import { invokeWrapper } from './wrapper';

export const simulateBondGateway = async (args: any) => invokeWrapper<FeeDetails>('simulate_bond_gateway', args);

export const simulateUnbondGateway = async (args: any) => invokeWrapper<FeeDetails>('simulate_unbond_gateway', args);

export const simulateBondMixnode = async (args: any) => invokeWrapper<FeeDetails>('simulate_bond_mixnode', args);

export const simulateUnbondMixnode = async (args: any) => invokeWrapper<FeeDetails>('simulate_unbond_mixnode', args);

export const simulateUpdateMixnode = async (args: any) => invokeWrapper<FeeDetails>('simulate_update_mixnode', args);

export const simulateDelegateToMixnode = async (args: any) =>
  invokeWrapper<FeeDetails>('simulate_delegate_to_mixnode', args);

export const simulateUndelegateFromMixnode = async (args: any) =>
  invokeWrapper<FeeDetails>('simulate_undelegate_from_mixnode,', args);

export const simulateVestingBondGateway = async (args: any) =>
  invokeWrapper<FeeDetails>('simulate_vesting_bond_gateway', args);

export const simulateVestingUnbondGateway = async (args: any) =>
  invokeWrapper<FeeDetails>('simulate_vesting_unbond_gateway', args);

export const simulateVestingBondMixnode = async (args: any) =>
  invokeWrapper<FeeDetails>('simulate_vesting_bond_mixnode', args);

export const simulateVestingUnbondMixnode = async (args: any) =>
  invokeWrapper<FeeDetails>('simulate_vesting_unbond_mixnode', args);

export const simulateVestingUpdateMixnode = async (args: any) =>
  invokeWrapper<FeeDetails>('simulate_vesting_update_mixnode', args);

export const simulateWithdrawVestedCoins = async ({ amount }: { amount: MajorCurrencyAmount }) =>
  invokeWrapper<FeeDetails>('simulate_withdraw_vested_coins', { amount });
