import { invoke } from '@tauri-apps/api';
import { Account, Network } from '../types';

export const selectNetwork = async (network: Network): Promise<Account> => invoke('switch_network', { network });
