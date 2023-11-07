import { Account } from '@nymproject/types';
import { Network } from '../types';
import { invokeWrapper } from './wrapper';

export const selectNetwork = async (network: Network) => invokeWrapper<Account>('switch_network', { network });

export const getSelectedValidatorUrl = async (network: Network) =>
  invokeWrapper<string | null>('get_selected_nyxd_url', { network });

export const getDefaultValidatorUrl = async (network: Network) =>
  invokeWrapper<string | null>('get_default_nyxd_url', { network });

export const setSelectedValidatorUrl = async (args: { network: Network; url: string }) =>
  invokeWrapper<void>('select_nyxd_url', args);

export const resetValidatorUrl = async (network: Network) => invokeWrapper<void>('reset_nyxd_url', { network });
