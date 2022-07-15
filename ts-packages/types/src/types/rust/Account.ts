import type { CurrencyDenom } from './CurrencyDenom';

export interface Account {
  client_address: string;
  base_mix_denom: string;
  display_mix_denom: CurrencyDenom;
}
