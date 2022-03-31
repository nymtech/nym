/* eslint-disable no-console */
import { config } from '../../config';

export const Console = {
  log: (output: string) => (config.IS_DEV_MODE ? console.log(output) : undefined),
  warn: (output: string) => (config.IS_DEV_MODE ? console.warn(output) : undefined),
  error: (output: string) => console.error(output),
};
