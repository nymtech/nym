import { Coin } from '@cosmjs/proto-signing';

export interface ISimulateClient {
  simulateSend(signingAddress: string, from: string, to: string, amount: Coin[]): Promise<number>;
}
