export const basicRawCoinValueValidation = (rawAmount: string): boolean => {
  const amountFloat = parseFloat(rawAmount);

  // it cannot have more than 6 decimal places
  if (amountFloat !== parseInt(amountFloat.toFixed(6), Number(10))) {
    return false;
  }

  // it cannot be larger than the total supply
  if (amountFloat > 1_000_000_000_000_000) {
    return false;
  }

  // it can't be lower than one micro coin
  return amountFloat >= 0.000001;
};
