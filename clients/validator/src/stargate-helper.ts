import axios from 'axios';
import { GasPrice } from '@cosmjs/stargate';

export function nymGasPrice(prefix: string): GasPrice {
  if (typeof prefix === 'string') {
    return GasPrice.fromString(`0.025u${prefix}`); // TODO: ideally this ugly conversion shouldn't be hardcoded here.
  }
  else {
    throw new Error(`${prefix} is not of type string`);
  }
}

export const downloadWasm = async (url: string): Promise<Uint8Array> => {
  const r = await axios.get(url, { responseType: 'arraybuffer' });
  if (r.status !== 200) {
    throw new Error(`Download error: ${r.status}`);
  }
  return r.data;
};
