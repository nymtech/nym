import { Coin } from "./coin";

export interface PledgeData {
  amount: Coin;
  block_time: bigint;
}