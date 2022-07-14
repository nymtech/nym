import type { CurrencyDenom } from './CurrencyDenom';

export interface Account {
  client_address: string;
  mix_denom: CurrencyDenom;
}
