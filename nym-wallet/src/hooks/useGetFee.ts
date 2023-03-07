import { DecCoin, FeeDetails } from '@nymproject/types';
import { useState } from 'react';
import { Console } from 'src/utils/console';
import { getCustomFees } from '../requests';

export function useGetFee() {
  const [fee, setFee] = useState<FeeDetails>();
  const [isFeeLoading, setIsFeeLoading] = useState(false);
  const [feeError, setFeeError] = useState<string>();

  async function getFee<T>(operation: (args: T) => Promise<FeeDetails>, args: T) {
    try {
      setIsFeeLoading(true);
      const simulatedFee = await operation(args);
      setFee(simulatedFee);
    } catch (e) {
      Console.error(e);
      setFeeError(e as string);
    }
    setIsFeeLoading(false);
  }

  async function setFeeManually(amount: DecCoin) {
    try {
      setIsFeeLoading(true);
      const fees = await getCustomFees({ feesAmount: amount });
      setFee(fees);
    } catch (e) {
      Console.error(e);
      setFeeError(e as string);
    }
    setIsFeeLoading(false);
  }

  const resetFeeState = () => {
    setFee(undefined);
    setIsFeeLoading(false);
    setFeeError(undefined);
  };

  return {
    fee,
    isFeeLoading,
    feeError,
    getFee,
    setFeeManually,
    resetFeeState,
  };
}
