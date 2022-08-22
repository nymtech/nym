import { AppEnv } from 'src/types';
import { MixNodeCostParams } from '@nymproject/types';
import { invokeWrapper } from './wrapper';

export const getEnv = async () => invokeWrapper<AppEnv>('get_env');

export const tryConvertIdentityToMixId = async (mixIdentity: string) =>
  invokeWrapper<number | null>('try_convert_pubkey_to_mix_id', { mixIdentity });

export const getDefaultMixnodeCostParams = async (profit_margin_percent: string) =>
  invokeWrapper<MixNodeCostParams>('default_mixnode_cost_params', { profit_margin_percent });
