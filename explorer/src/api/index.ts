import {
  GATEWAYS_API,
  MIXNODES_API,
  VALIDATORS_API,
  BLOCK_API,
  COUNTRY_DATA_API,
} from './constants';

import { MixNodeResponse, GatewayResponse, ValidatorsResponse, CountryDataResponse } from '../typeDefs/node-status-api-client'

export class Api {

  static fetchMixnodes = async (): Promise<MixNodeResponse> => {
    try {
      const res = await fetch(MIXNODES_API);
      const json = await res.json();
      return json;
    } catch (error: any) {
      console.log('error ', error);
      throw error;
    }
  };

  static fetchGateways = async (): Promise<GatewayResponse> => {
    try {
      const res = await fetch(GATEWAYS_API);
      const json = await res.json();
      return json;
    } catch (error: any) {
      console.log('error ', error);
      return error;
    }
  };

  static fetchValidators = async (): Promise<ValidatorsResponse> => {
    try {
      const res = await fetch(VALIDATORS_API);
      const json = await res.json();
      return json.result;
    } catch (error: any) {
      console.log('error ', error);
      return error;
    }
  };

  static fetchBlock = async (): Promise<number> => {
    try {
      const res = await fetch(BLOCK_API);
      const json = await res.json();
      const { height } = json.result.block.header;
      return height;
    } catch (error: any) {
      console.log('error ', error);
      return error;
    }
  };

  static fetchCountryData = async (): Promise<CountryDataResponse> => {
    try {
      let arr: CountryDataResponse = [];
      const res = await fetch(COUNTRY_DATA_API);
      const json = await res.json();
      Object.keys(json)
        .forEach(ISO3 => { 
          arr.push({ ISO3, nodes: json[ISO3]}) 
        });

      return arr;
    } catch (error: any) {
      console.log('error ', error);
      return error;
    }
  };

}
