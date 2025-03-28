import {
  Fee,
  DecCoin,
  SendTxResult,
  TransactionExecuteResult,
  MixNodeConfigUpdate,
  NodeConfigUpdate,
  GatewayConfigUpdate,
  NodeCostParams,
} from '@nymproject/types';
import { TBondGatewayArgs, TBondGatewaySignatureArgs } from '../types';
import { invokeWrapper } from './wrapper';

export const bondGateway = async (args: TBondGatewayArgs) =>
  invokeWrapper<TransactionExecuteResult>('bond_gateway', args);

export const generateGatewayMsgPayload = async (args: Omit<TBondGatewaySignatureArgs, 'tokenPool'>) =>
  invokeWrapper<string>('generate_gateway_bonding_msg_payload', args);

export const updateMixnodeConfig = async (update: MixNodeConfigUpdate, fee?: Fee) =>
  invokeWrapper<TransactionExecuteResult>('update_mixnode_config', { update, fee });

export const updateNymNodeConfig = async (update: NodeConfigUpdate, fee?: Fee) =>
  invokeWrapper<TransactionExecuteResult>('update_nymnode_config', { update, fee });

export const updateNymNodeParams = async (newCosts: NodeCostParams, fee?: Fee) => {
  const result = await invokeWrapper<TransactionExecuteResult>('update_nymnode_cost_params', {
    newCosts,
    fee,
  });
  return result;
};

export const updateGatewayConfig = async (update: GatewayConfigUpdate, fee?: Fee) =>
  invokeWrapper<TransactionExecuteResult>('update_gateway_config', { update, fee });

export const send = async (args: { amount: DecCoin; address: string; memo: string; fee?: Fee }) =>
  invokeWrapper<SendTxResult>('send', args);

export const migrateVestedMixnode = async () => invokeWrapper<TransactionExecuteResult>('migrate_vested_mixnode');
