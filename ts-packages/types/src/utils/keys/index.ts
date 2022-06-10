import bs58 from 'bs58';

export const validateKey = (key?: string, bytesLength: number = 32): boolean => {
  if (!key) {
    return false;
  }

  // it must be a valid base58 key
  try {
    const bytes = bs58.decode(key);
    // of length 32
    return bytes.length === bytesLength;
  } catch (e) {
    // eslint-disable-next-line no-console
    console.error(e);
    return false;
  }
};
