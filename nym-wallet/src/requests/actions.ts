import {
  Fee,
  DecCoin,
  SendTxResult,
  TransactionExecuteResult,
  MixNodeConfigUpdate,
  GatewayConfigUpdate,
} from '@nymproject/types';
import { TBondGatewayArgs, TBondGatewaySignatureArgs, TNodeConfigUpdateArgs } from '../types';
import { invokeWrapper } from './wrapper';

export const bondGateway = async (args: TBondGatewayArgs) =>
  invokeWrapper<TransactionExecuteResult>('bond_gateway', args);

export const generateGatewayMsgPayload = async (args: Omit<TBondGatewaySignatureArgs, 'tokenPool'>) =>
  invokeWrapper<string>('generate_gateway_bonding_msg_payload', args);

export const updateMixnodeConfig = async (update: MixNodeConfigUpdate, fee?: Fee) =>
  invokeWrapper<TransactionExecuteResult>('update_mixnode_config', { update, fee });

export const updateNymNodeConfig = async (update: TNodeConfigUpdateArgs, fee?: Fee) =>
  invokeWrapper<TransactionExecuteResult>('update_nymnode_config', { update, fee });

export const updateGatewayConfig = async (update: GatewayConfigUpdate, fee?: Fee) =>
  invokeWrapper<TransactionExecuteResult>('update_gateway_config', { update, fee });

export const send = async (args: { amount: DecCoin; address: string; memo: string; fee?: Fee }) =>
  invokeWrapper<SendTxResult>('send', args);

export const migrateVestedMixnode = async () => invokeWrapper<TransactionExecuteResult>('migrate_vested_mixnode');
