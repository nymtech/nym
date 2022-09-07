import { invokeWrapper } from './wrapper';

export const sign = async (message: string): Promise<string> => invokeWrapper<string>('sign', { message });

export const verify = async (publicKeyAsJson: string, signatureAsHex: string, message: string): Promise<string> =>
  invokeWrapper<string>('verify', { publicKeyAsJson, signatureAsHex, message });
