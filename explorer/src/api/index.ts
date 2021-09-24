import {
  GATEWAYS_API,
  MIXNODES_API,
  VALIDATORS_API,
  BLOCK_API,
  COUNTRY_DATA_API,
} from './constants';

import { MixNodeResponse, GatewayResponse, ValidatorsResponse, CountryDataResponse } from '../typeDefs/explorer-api'

export class Api {

  static fetchMixnodes = async (): Promise<MixNodeResponse> => {
    const res = await fetch(MIXNODES_API);
    return await res.json();
  };

  static fetchGateways = async (): Promise<GatewayResponse> => {
    const res = await fetch(GATEWAYS_API);
    return await res.json();
  };

  static fetchValidators = async (): Promise<ValidatorsResponse> => {
    const res = await fetch(VALIDATORS_API);
    const json = await res.json();
    return json.result;
  };
  
  static fetchBlock = async (): Promise<number> => {
    const res = await fetch(BLOCK_API);
    const json = await res.json();
    const { height } = json.result.block.header;
    return height;
  };

  static fetchCountryData = async (): Promise<CountryDataResponse> => {
      let arr: CountryDataResponse = [];
      const res = await fetch(COUNTRY_DATA_API);
      const json = await res.json();
      Object.keys(json)
        .forEach(ISO3 => { 
          arr.push({ ISO3, nodes: json[ISO3]}) 
        });
      return arr;
  };
}
