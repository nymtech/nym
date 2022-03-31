import type { Denom } from './denom';

export interface Account {
  contract_address: string;
  client_address: string;
  denom: Denom;
}
