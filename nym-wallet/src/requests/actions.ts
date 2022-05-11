import { MajorCurrencyAmount, SendTxResult, TransactionExecuteResult } from '@nymproject/types';
import { EnumNodeType, TBondArgs, TBondGatewayArgs, TBondMixNodeArgs } from '../types';
import { invokeWrapper } from './wrapper';

export const bondGateway = async (args: TBondGatewayArgs) =>
  invokeWrapper<TransactionExecuteResult>('bond_gateway', args);

export const unbondGateway = async () => invokeWrapper<TransactionExecuteResult>('unbond_gateway');

export const bondMixNode = async (args: TBondMixNodeArgs) =>
  invokeWrapper<TransactionExecuteResult>('bond_mixnode', args);

export const unbondMixNode = async () => invokeWrapper<TransactionExecuteResult>('unbond_mixnode');

export const updateMixnode = async (profitMarginPercent: number) =>
  invokeWrapper<TransactionExecuteResult>('update_mixnode', { profitMarginPercent });

export const send = async (args: { amount: MajorCurrencyAmount; address: string; memo: string }) =>
  invokeWrapper<SendTxResult>('send', args);

export const unbond = async (type: EnumNodeType) => {
  if (type === EnumNodeType.mixnode) return unbondMixNode();
  return unbondGateway();
};

export const bond = async (args: TBondArgs) => {
  const { type, ...other } = args;
  if (type === EnumNodeType.mixnode) return bondMixNode(other as TBondMixNodeArgs);
  return bondGateway(other as TBondGatewayArgs);
};
