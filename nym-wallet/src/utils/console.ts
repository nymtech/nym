/* eslint-disable no-console */
import { config } from '../config';

export const Console = {
  log: (message?: any, ...optionalParams: any[]) =>
    config.IS_DEV_MODE ? console.log(message, ...optionalParams) : undefined,
  warn: (message?: any, ...optionalParams: any[]) =>
    config.IS_DEV_MODE ? console.warn(message, ...optionalParams) : undefined,
  error: (message?: any, ...optionalParams: any[]) => console.error(message, ...optionalParams),
};
