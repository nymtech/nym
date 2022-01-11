import {
  BLOCK_API,
  COUNTRY_DATA_API,
  GATEWAYS_API,
  MIXNODE_API,
  MIXNODE_PING,
  MIXNODES_API,
  OVERVIEW_API,
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
  MixnodeStatus,
  StatsResponse,
  StatusResponse,
  SummaryOverviewResponse,
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
  static fetchOverviewSummary = async (): Promise<SummaryOverviewResponse> => {
    const cache = getFromCache('overview-summary');
    if (cache) {
      return cache;
    }
    const res = await fetch(`${OVERVIEW_API}/summary`);
    const json = await res.json();
    storeInCache('overview-summary', JSON.stringify(json));
    return json;
  };

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

  static fetchMixnodesActiveSetByStatus = async (
    status: MixnodeStatus,
  ): Promise<MixNodeResponse> => {
    const cachedMixnodes = getFromCache(`mixnodes-${status}`);
    if (cachedMixnodes) {
      return cachedMixnodes;
    }
    const res = await fetch(`${MIXNODES_API}/active-set/${status}`);
    const json = await res.json();
    storeInCache(`mixnodes-${status}`, JSON.stringify(json));
    return json;
  };

  static fetchMixnodeByID = async (
    id: string,
  ): Promise<MixNodeResponseItem | undefined> => {
    const response = await fetch(`${MIXNODE_API}/${id}`);

    // when the mixnode is not found, returned undefined
    if (response.status === 404) {
      return undefined;
    }

    return response.json();
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
