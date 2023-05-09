import { useState } from 'react';
import { unymToNym } from 'src/utils/coin';

export const useGetFee = () => {
  const [fee, setFee] = useState<number>();
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string>();

  const getFee = async <T>(txReq: (args: T) => Promise<number | undefined>, args: T) => {
    setError(undefined);
    setIsLoading(true);

    try {
      const txFee = await txReq(args);

      if (txFee) {
        const txFeeInNyms = unymToNym(txFee);
        setFee(txFeeInNyms);
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
  };

  return { fee, getFee, isLoading, error };
};
