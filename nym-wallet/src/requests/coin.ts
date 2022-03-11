import { invoke } from '@tauri-apps/api';
import { Coin } from '../types';

export const minorToMajor = async (amount: string): Promise<Coin> => invoke('minor_to_major', { amount });

export const majorToMinor = async (amount: string): Promise<Coin> => invoke('major_to_minor', { amount });
