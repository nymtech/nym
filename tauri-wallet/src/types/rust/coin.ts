import { Denom } from "./denom";

export interface Coin {
  amount: string;
  denom: Denom;
}