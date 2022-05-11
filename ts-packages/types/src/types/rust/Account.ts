import type { CurrencyDenom } from './CurrencyDenom';

export interface Account {
  contract_address: string;
  client_address: string;
  denom: CurrencyDenom;
}
