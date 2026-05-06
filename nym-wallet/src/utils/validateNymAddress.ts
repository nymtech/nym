/** NYM bech32 sending address: `n1` prefix, 40 chars total, lowercase alphanumeric body. */
export const validateNymAddress = (address: string): boolean => {
  if (!address) return false;
  if (!address.startsWith('n1')) return false;
  if (address.length !== 40) return false;
  return /^[a-z0-9]+$/.test(address);
};
