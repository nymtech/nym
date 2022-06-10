import type { Gateway } from './Gateway';
import type { MajorCurrencyAmount } from './Currency';

export interface GatewayBond {
  pledge_amount: MajorCurrencyAmount;
  owner: string;
  block_height: bigint;
  gateway: Gateway;
  proxy: string | null;
}
