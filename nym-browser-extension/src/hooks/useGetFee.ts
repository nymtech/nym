import Big from 'big.js';
import { useState } from 'react';
import { unymToNym } from 'src/utils/coin';

export type Fee = { nym: number; unym: number };

export function useGetFee() {
  const [fee, setFee] = useState<Fee>();
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string>();

  async function getFee<T>(txReq: (args: T) => Promise<number | undefined>, args: T) {
    setError(undefined);
    setIsLoading(true);

    try {
      const txFee = await txReq(args);

      if (txFee) {
        const feeWithMultiplyer = Big(txFee).mul(1);
        console.log(fee);

        const txFeeInNyms = unymToNym(feeWithMultiplyer);

        setFee({ nym: Number(txFeeInNyms), unym: Number(feeWithMultiplyer) });
      }

      if (!txFee) {
        setError('Unable to calculate fee');
      }
    } catch (e) {
      console.error(e);
      setError(`Unable to get estimated fee: ${e}`);
    } finally {
      setIsLoading(false);
    }
  }

  return { fee, getFee, isLoading, error };
}
