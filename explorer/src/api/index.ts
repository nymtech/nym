import {
  BLOCK_API,
  COUNTRY_DATA_API,
  GATEWAYS_API,
  MIXNODE_API,
  MIXNODE_PING,
  MIXNODES_API,
  UPTIME_STORY_API,
  VALIDATORS_API,
} from './constants';

import {
  CountryDataResponse,
  DelegationsResponse,
  GatewayResponse,
  MixNodeDescriptionResponse,
  MixNodeResponse,
  MixNodeResponseItem,
  StatsResponse,
  StatusResponse,
  UptimeStoryResponse,
  ValidatorsResponse,
} from '../typeDefs/explorer-api';

function getFromCache(key: string) {
  const ts = Number(localStorage.getItem('ts'));
  const hasExpired = Date.now() - ts > 5000;
  const curr = localStorage.getItem(key);
  if (curr && !hasExpired) {
    return JSON.parse(curr);
  }
  return undefined;
}

function storeInCache(key: string, data: any) {
  localStorage.setItem(key, data);
}
export class Api {
  static fetchMixnodes = async (): Promise<MixNodeResponse> => {
    const cachedMixnodes = getFromCache('mixnodes');
    if (cachedMixnodes) {
      return cachedMixnodes;
    }
    const res = await fetch(MIXNODES_API);
    const json = await res.json();
    storeInCache('mixnodes', JSON.stringify(json));
    storeInCache('ts', Date.now());
    return json;
  };

  static fetchMixnodeByID = async (
    id: string,
  ): Promise<MixNodeResponseItem | undefined> => {
    // TODO: replace with call to API to get single mix node by id
    const allMixnodes = await Api.fetchMixnodes();
    return allMixnodes.find((item) => item.mix_node.identity_key === id);
  };

  static fetchGateways = async (): Promise<GatewayResponse> => {
    const res = await fetch(GATEWAYS_API);
    return res.json();
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
    const result: CountryDataResponse = {};
    const res = await fetch(COUNTRY_DATA_API);
    const json = await res.json();
    Object.keys(json).forEach((ISO3) => {
      result[ISO3] = { ISO3, nodes: json[ISO3] };
    });
    return result;
  };

  static fetchDelegationsById = async (
    id: string,
  ): Promise<DelegationsResponse> =>
    (await fetch(`${MIXNODE_API}/${id}/delegations`)).json();

  static fetchStatsById = async (id: string): Promise<StatsResponse> =>
    (await fetch(`${MIXNODE_API}/${id}/stats`)).json();

  static fetchMixnodeDescriptionById = async (
    id: string,
  ): Promise<MixNodeDescriptionResponse> =>
    (await fetch(`${MIXNODE_API}/${id}/description`)).json();

  static fetchStatusById = async (id: string): Promise<StatusResponse> =>
    (await fetch(`${MIXNODE_PING}/${id}`)).json();

  static fetchUptimeStoryById = async (
    id: string,
  ): Promise<UptimeStoryResponse> =>
    (await fetch(`${UPTIME_STORY_API}/${id}/history`)).json();
}
