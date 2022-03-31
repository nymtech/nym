import { invoke } from '@tauri-apps/api';
import { TauriContractStateParams } from '../types';

export const getContractParams = async (): Promise<TauriContractStateParams> => {
  const res: TauriContractStateParams = await invoke('get_contract_settings');
  return res;
};

export const setContractParams = async (params: TauriContractStateParams): Promise<TauriContractStateParams> => {
  const res: TauriContractStateParams = await invoke('update_contract_settings', { params });
  return res;
};
