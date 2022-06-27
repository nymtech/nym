import { Decimal } from '@cosmjs/math';
import { MajorCurrencyAmount } from '@nymproject/types';

export const toMajorCurrency = (amount: string, denom: string): MajorCurrencyAmount => {
  if (denom[0].toLowerCase() !== 'u') {
    return {
      amount,
      denom: denom as any,
    };
  }
  const decimal = Decimal.fromAtomics(amount, 6);
  return {
    amount: decimal.toString(),
    denom: denom.slice(1) as any,
  };
};

export const toMajorCurrencyFromCoin = (coin?: { amount: string; denom: string }): MajorCurrencyAmount | undefined => {
  if (!coin) {
    return undefined;
  }
  return toMajorCurrency(coin.amount, coin.denom);
};
