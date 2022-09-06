import { invokeWrapper } from './wrapper';

export const sign = async (message: string): Promise<string> => invokeWrapper<string>('sign', { message });
