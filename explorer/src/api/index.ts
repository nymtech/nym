import {
  GATEWAYS_API,
  MIXNODES_API,
  VALIDATORS_API,
  BLOCK_API,
} from './constants';

export class Api {
  static fetchMixnodes = async () => {
    try {
      const res = await fetch(MIXNODES_API);
      const json = await res.json();
      return json;
    } catch (error: any) {
      console.log('error ', error);
      throw error.message;
    }
  };

  static fetchGateways = async () => {
    try {
      const res = await fetch(GATEWAYS_API);
      const json = await res.json();
      return json;
    } catch (error) {
      console.log('error ', error);
      return error;
    }
  };

  static fetchValidators = async () => {
    try {
      const res = await fetch(VALIDATORS_API);
      const json = await res.json();
      return json.result.count;
    } catch (error) {
      console.log('error ', error);
      return error;
    }
  };

  static fetchBlock = async () => {
    try {
      const res = await fetch(BLOCK_API);
      const json = await res.json();
      const { height } = json.result.block.header;
      return height;
    } catch (error) {
      console.log('error ', error);
      return error;
    }
  };
}
