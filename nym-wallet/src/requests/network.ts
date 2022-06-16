import { Account } from '@nymproject/types';
import { Network } from 'src/types';
import { invokeWrapper } from './wrapper';

export const selectNetwork = async (network: Network) => invokeWrapper<Account>('switch_network', { network });
