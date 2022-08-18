import { Fee, DecCoin, SendTxResult, TransactionExecuteResult } from '@nymproject/types';
import { EnumNodeType, TBondGatewayArgs, TBondMixNodeArgs } from '../types';
import { invokeWrapper } from './wrapper';

export const bondGateway = async (args: TBondGatewayArgs) =>
  invokeWrapper<TransactionExecuteResult>('bond_gateway', args);

export const unbondGateway = async (fee?: Fee) => invokeWrapper<TransactionExecuteResult>('unbond_gateway', { fee });

export const bondMixNode = async (args: TBondMixNodeArgs) =>
  invokeWrapper<TransactionExecuteResult>('bond_mixnode', args);

export const unbondMixNode = async (fee?: Fee) => invokeWrapper<TransactionExecuteResult>('unbond_mixnode', { fee });

export const updateMixnode = async (profitMarginPercent: number, fee?: Fee) =>
  invokeWrapper<TransactionExecuteResult>('update_mixnode', { profitMarginPercent, fee });

export const send = async (args: { amount: DecCoin; address: string; memo: string; fee?: Fee }) =>
  invokeWrapper<SendTxResult>('send', args);

export const unbond = async (type: EnumNodeType) => {
  if (type === EnumNodeType.mixnode) return unbondMixNode();
  return unbondGateway();
};
