import { invoke } from '@tauri-apps/api';
import { Coin } from '../types';

export const minorToMajor = async (amount: string): Promise<Coin> => {
  const res: Coin = await invoke('minor_to_major', { amount });
  return res;
};

export const majorToMinor = async (amount: string): Promise<Coin> => {
  const res: Coin = await invoke('major_to_minor', { amount });
  return res;
};
