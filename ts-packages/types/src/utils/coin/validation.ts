export const isValidRawCoin = (rawAmount: string): boolean => {
  const amountFloat = parseFloat(rawAmount);

  // if value is a decimal it cannot have more than 6 decimal places
  if (amountFloat % 1 > 0) {
    const [_, numsAfterDecimal] = rawAmount.split('.');

    if (+numsAfterDecimal.length > 6) {
      return false;
    }
  }

  // it cannot be larger than the total supply
  if (amountFloat > 1_000_000_000) {
    return false;
  }

  console.log(amountFloat);
  // it can't be lower than one micro coin
  if (amountFloat < 0.000001) {
    return false;
  }

  return true;
};
