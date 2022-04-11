import { invoke } from '@tauri-apps/api';
import { Network } from '../types';

import {
    ValidatorUrls
} from '../types';

export const getValidatorUrls = async (network: Network): Promise<ValidatorUrls> => {
    const res: ValidatorUrls = await invoke('get_validator_nymd_urls', { network });
    return res;
};

export const selectValidatorNymdUrl = async (validator: string, network: Network): Promise<void> => {
    const res: void = await invoke('select_validator_nymd_url', { url: validator, network });
    return res;
};
