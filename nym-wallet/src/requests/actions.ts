import { MajorCurrencyAmount, SendTxResult, TransactionExecuteResult } from '@nymproject/types';
import { Fee } from '@nymproject/types/dist/types/rust/Fee';
import { EnumNodeType, TBondGatewayArgs, TBondMixNodeArgs } from '../types';
import { invokeWrapper } from './wrapper';

export const bondGateway = async (args: TBondGatewayArgs) =>
  invokeWrapper<TransactionExecuteResult>('bond_gateway', args);

export const unbondGateway = async (fee?: Fee) => invokeWrapper<TransactionExecuteResult>('unbond_gateway', { fee });

export const bondMixNode = async (args: TBondMixNodeArgs) =>
  invokeWrapper<TransactionExecuteResult>('bond_mixnode', args);

export const unbondMixNode = async (fee?: Fee) => invokeWrapper<TransactionExecuteResult>('unbond_mixnode', { fee });

export const updateMixnode = async (profitMarginPercent: number) =>
  invokeWrapper<TransactionExecuteResult>('update_mixnode', { profitMarginPercent });

export const send = async (args: { amount: MajorCurrencyAmount; address: string; memo: string }) =>
  invokeWrapper<SendTxResult>('send', args);

export const unbond = async (type: EnumNodeType) => {
  if (type === EnumNodeType.mixnode) return unbondMixNode();
  return unbondGateway();
};
