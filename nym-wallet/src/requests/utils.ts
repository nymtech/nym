import { AppEnv } from 'src/types';
import { NodeCostParams } from '@nymproject/types';
import { invokeWrapper } from './wrapper';

export const getEnv = async () => invokeWrapper<AppEnv>('get_env');

export const tryConvertIdentityToMixId = async (mixIdentity: string) =>
  invokeWrapper<number | null>('try_convert_pubkey_to_mix_id', { mixIdentity });

export const getDefaultNodeCostParams = async (profitMarginPercent: string) =>
  invokeWrapper<NodeCostParams>('default_mixnode_cost_params', { profitMarginPercent });
