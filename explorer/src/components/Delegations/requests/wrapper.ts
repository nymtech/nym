import { invoke } from '@tauri-apps/api';
import { config } from '../config';
import { Console } from '../utils/console';

export async function invokeWrapper<T>(operationName: string, args?: any): Promise<T> {
  const res = await invoke<T>(operationName, args);
  if (config.LOG_TAURI_OPERATIONS) {
    const argsToLog: any = {};
    if (args) {
      Object.keys(args).forEach((key) => {
        // check if the key should be excluded from logs
        if (!['mnemonic', 'password', 'currentPassword', 'newPassword'].includes(key)) {
          argsToLog[key] = args[key];
        }
      });
    }
    Console.log({ operationName, argsToLog, res });
  }
  return res;
}
