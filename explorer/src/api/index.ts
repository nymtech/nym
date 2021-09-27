import {
  GATEWAYS_API,
  MIXNODES_API,
  VALIDATORS_API,
  BLOCK_API,
  COUNTRY_DATA_API,
} from './constants';

import { MixNodeResponse, GatewayResponse, ValidatorsResponse, CountryDataResponse, MixNodeResponseItem } from '../typeDefs/explorer-api'

function getFromCache(key: string) {
  const ts = Number(localStorage.getItem('ts'));
  const hasExpired = Date.now() - ts > 200000;
  const curr = localStorage.getItem(key);
  if (curr && !hasExpired) {
    return JSON.parse(curr);
  } else {
    return undefined;
  }
}

function storeInCache(key: string, data: any) {
  localStorage.setItem(key, data);
}

function clearCache(key: string) {
  return localStorage.removeItem(key);
}
export class Api {

  static fetchMixnodes = async (): Promise<MixNodeResponse> => {
    let cachedMixnodes = getFromCache('mixnodes');
    if (cachedMixnodes) {
      return cachedMixnodes;
    } else {
      const res = await fetch(MIXNODES_API);
      const json = await res.json();
      storeInCache('mixnodes', JSON.stringify(json))
      storeInCache('ts', Date.now())
      return json;
    }
  };

  static fetchMixnodeByID = async (id: string): Promise<MixNodeResponseItem | undefined> => {
    const allMixnodes: MixNodeResponse = await Api.fetchMixnodes();
    const matchedByID = allMixnodes.filter(eachRecord => eachRecord.mix_node.identity_key === id);
    return matchedByID.length && matchedByID[0] || undefined;
  }

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
