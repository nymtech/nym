import { decode } from 'bech32';

export const validateWalletAddress = (address?: string, prefix: string = 'n', logErrorToConsole = false): boolean => {
  if (!address) {
    return false;
  }

  if (address.length < 1) {
    return false;
  }

  if (!address.startsWith(prefix)) {
    return false;
  }

  try {
    // try to decode the address
    decode(address);
  } catch (e) {
    if (logErrorToConsole) {
      // eslint-disable-next-line no-console
      console.error('Failed to decode address', e);
    }
    return false;
  }

  return true;
};
