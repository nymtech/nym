import {
  Fee,
  DecCoin,
  SendTxResult,
  TransactionExecuteResult,
  MixNodeConfigUpdate,
  MixNodeCostParams,
} from '@nymproject/types';
import { EnumNodeType, TBondGatewayArgs, TBondMixNodeArgs, TBondMoreArgs } from '../types';
import { invokeWrapper } from './wrapper';

export const bondGateway = async (args: TBondGatewayArgs) =>
  invokeWrapper<TransactionExecuteResult>('bond_gateway', args);

export const unbondGateway = async (fee?: Fee) => invokeWrapper<TransactionExecuteResult>('unbond_gateway', { fee });

export const bondMixNode = async (args: TBondMixNodeArgs) =>
  invokeWrapper<TransactionExecuteResult>('bond_mixnode', args);

export const unbondMixNode = async (fee?: Fee) => invokeWrapper<TransactionExecuteResult>('unbond_mixnode', { fee });

export const updateMixnodeCostParams = async (newCosts: MixNodeCostParams, fee?: Fee) =>
  invokeWrapper<TransactionExecuteResult>('update_mixnode_cost_params', { newCosts, fee });

export const updateMixnodeConfig = async (update: MixNodeConfigUpdate, fee?: Fee) =>
  invokeWrapper<TransactionExecuteResult>('update_mixnode_config', { update, fee });

export const send = async (args: { amount: DecCoin; address: string; memo: string; fee?: Fee }) =>
  invokeWrapper<SendTxResult>('send', args);

export const unbond = async (type: EnumNodeType) => {
  if (type === EnumNodeType.mixnode) return unbondMixNode();
  return unbondGateway();
};

export const bondMore = async (args: TBondMoreArgs) => invokeWrapper<TransactionExecuteResult>('pledge_more', args);
