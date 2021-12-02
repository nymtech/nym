import { Coin } from "./coin";

export interface TransactionDetails {
  amount: Coin;
  from_address: string;
  to_address: string;
}