import type { DecCoin } from "@nymproject/types";
import Big from "big.js";

export const isValidRawCoin = (rawAmount: string): boolean => {
  const amountFloat = Number.parseFloat(rawAmount);

  // if value is a decimal it cannot have more than 6 decimal places
  if (amountFloat % 1 > 0) {
    // eslint-disable-next-line @typescript-eslint/no-unused-vars,@typescript-eslint/naming-convention
    const [_, numsAfterDecimal] = rawAmount.split(".");

    if (+numsAfterDecimal.length > 6) {
      return false;
    }
  }

  // it cannot be larger than the total supply
  if (amountFloat > 1_000_000_000) {
    return false;
  }

  // it can't be lower than one micro coin
  if (amountFloat < 0.000001) {
    return false;
  }

  return true;
};

export const unymToNym = (unym: string | number | Big, dp = 4) => {
  try {
    const nym = Big(unym).div(1_000_000).toFixed(dp);
    return nym;
  } catch (e: unknown) {
    console.warn(`${unym} not a valid decimal number: ${e}`);
  }
};

export const validateAmount = async (
  majorAmountAsString: DecCoin["amount"],
  minimumAmountAsString: DecCoin["amount"]
): Promise<boolean> => {
  // tests basic coin value requirements, like no more than 6 decimal places, value lower than total supply, etc
  if (!Number(majorAmountAsString)) {
    return false;
  }

  if (!isValidRawCoin(majorAmountAsString)) {
    return false;
  }

  const majorValueFloat = Number.parseInt(majorAmountAsString, Number(10));

  return majorValueFloat >= Number.parseInt(minimumAmountAsString, Number(10));
};
