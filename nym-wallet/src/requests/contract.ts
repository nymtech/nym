import { invoke } from '@tauri-apps/api';
import { TauriContractStateParams } from '../types';

export const getContractParams = async (): Promise<TauriContractStateParams> => invoke('get_contract_settings');

export const setContractParams = async (params: TauriContractStateParams): Promise<TauriContractStateParams> =>
  invoke('update_contract_settings', { params });
