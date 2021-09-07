import { Coin } from "./coin";

export interface TransactionDetails {
  from_address: string;
  to_address: string;
  amount: Coin;
}