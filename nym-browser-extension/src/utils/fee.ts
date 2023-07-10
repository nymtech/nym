export const createFeeObject = (feeInUnyms?: number) => {
  if (!feeInUnyms) return undefined;

  return {
    amount: [{ amount: feeInUnyms.toString(), denom: 'unym' }],
    gas: '100000',
  };
};
