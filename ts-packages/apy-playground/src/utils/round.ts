/**
 * Reproduce the behaviour of Python's round method
 * @param value      The floating point number to round
 * @param decimals   The number of decimals to round to, e.g. 11.4999 to 2 decimals is 11.50
 */
export const round = (value: number, decimals: number = 0): number => {
  if (decimals === 0) {
    return Math.round(value);
  }
  const pow = 10 ** decimals;
  return +Math.round(value * pow) / pow;
  // return +(Math.round(Number.parseFloat(value + `e+${decimals}`))  + `e-${decimals}`)
};

/**
 * Round returning 0 when value is undefined
 */
export const roundWithDefault = (value?: number, decimals: number = 0): number => {
  if (!value) {
    return 0;
  }
  return round(value, decimals);
};
