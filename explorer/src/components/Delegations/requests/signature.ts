import { invokeWrapper } from './wrapper';

export const sign = async (message: string): Promise<string> => invokeWrapper<string>('sign', { message });

export const verify = async (
  signatureAsHex: string,
  message: string,
  publicKeyAsJsonOrAccountAddress?: string | null,
): Promise<string> => invokeWrapper<string>('verify', { publicKeyAsJsonOrAccountAddress, signatureAsHex, message });
