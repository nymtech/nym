import Big from "big.js";

export const unymToNym = (unym: string | number | Big, dp = 4) => {
  try {
    const nym = Big(unym).div(1_000_000).toFixed(dp);
    return nym;
  } catch (e: unknown) {
    console.warn(`${unym} not a valid decimal number: ${e}`);
  }
};
