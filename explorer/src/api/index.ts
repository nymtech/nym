import { GATEWAYS_API, MIXNODES_API, VALIDATORS_API } from './constants';

export class Data {
  static fetchMixnodes = async () => {
    try {
      const res = await fetch(MIXNODES_API);
      const json = await res.json();
      return json;
    } catch (error) {
      return error;
    }
  };

  static fetchGateways = async () => {
    try {
      const res = await fetch(GATEWAYS_API);
      const json = await res.json();
      return json;
    } catch (error) {
      return error;
    }
  };

  static fetchValidators = async () => {
    try {
      const res = await fetch(VALIDATORS_API);
      const json = await res.json();
      return json.result.count;
    } catch (error) {
      return error;
    }
  };
}
