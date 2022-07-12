import type { Fee } from './Fee';
import type { DecCoin } from "./DecCoin";

export type FeeDetails = { amount: DecCoin | null; fee: Fee };
