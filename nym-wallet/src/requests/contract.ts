import { TauriContractStateParams } from 'src/types';
import { invokeWrapper } from './wrapper';

export const getContractParams = async () => invokeWrapper<TauriContractStateParams>('get_contract_settings');

export const setContractParams = async (params: TauriContractStateParams) =>
  invokeWrapper<TauriContractStateParams>('update_contract_settings', { params });
