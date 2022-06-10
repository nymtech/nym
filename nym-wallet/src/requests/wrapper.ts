import { invoke } from '@tauri-apps/api';
import { config } from '../config';
import { Console } from '../utils/console';

export async function invokeWrapper<T>(operationName: string, args?: any): Promise<T> {
  const res = await invoke<T>(operationName, args);
  if (config.LOG_TAURI_OPERATIONS) {
    Console.log({ operationName, args, res });
  }
  return res;
}
