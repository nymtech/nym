import keyBy from 'lodash/keyBy';
import {
  API_BASE_URL,
  BLOCK_API,
  COUNTRY_DATA_API,
  UPTIME_STORY_API_GATEWAY,
  MIXNODE_API,
  MIXNODE_PING,
  MIXNODES_API,
  OVERVIEW_API,
  UPTIME_STORY_API,
  VALIDATORS_API,
  SERVICE_PROVIDERS,
  TEMP_UNSTABLE_NYM_NODES,
  NYM_API_NODE_UPTIME,
  NYM_API_NODE_PERFORMANCE,
  TEMP_UNSTABLE_ACCOUNT,
  LEGACY_MIXNODES_API, LEGACY_GATEWAYS_API,
} from './constants';

import {
  CountryDataResponse,
  DelegationsResponse,
  UniqDelegationsResponse,
  GatewayReportResponse,
  UptimeStoryResponse,
  MixNodeDescriptionResponse,
  MixNodeResponse,
  MixNodeResponseItem,
  MixnodeStatus,
  MixNodeEconomicDynamicsStatsResponse,
  StatsResponse,
  StatusResponse,
  SummaryOverviewResponse,
  ValidatorsResponse,
  Environment,
  GatewayBondAnnotated,
  GatewayBond,
  DirectoryServiceProvider,
  LocatedGateway,
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
  localStorage.setItem('ts', Date.now().toString());
}

export class Api {
  static fetchOverviewSummary = async (): Promise<SummaryOverviewResponse> => {
    const cache = getFromCache('overview-summary');
    if (cache) {
      return cache;
    }
    const res = await fetch(`${OVERVIEW_API}/summary`);
    const json: SummaryOverviewResponse = await res.json();

    if (json.nymnodes?.roles) {
      json.mixnodes.count += json.nymnodes.roles.mixnode;
      json.gateways.count += json.nymnodes.roles.entry;
      json.gateways.count += Math.max(json.nymnodes.roles.exit_ipr, json.nymnodes.roles.exit_nr);
    }

    storeInCache('overview-summary', JSON.stringify(json));
    return json;
  };

  static fetchMixnodes = async (): Promise<MixNodeResponse> => {
    const cachedMixnodes = getFromCache('mixnodes');
    if (cachedMixnodes) {
      return cachedMixnodes;
    }

    const res = await fetch(LEGACY_MIXNODES_API);
    const json = await res.json();
    storeInCache('mixnodes', JSON.stringify(json));
    return json;
  };

  static fetchMixnodesActiveSetByStatus = async (status: MixnodeStatus): Promise<MixNodeResponse> => {
    const cachedMixnodes = getFromCache(`mixnodes-${status}`);
    if (cachedMixnodes) {
      return cachedMixnodes;
    }
    const res = await fetch(`${MIXNODES_API}/active-set/${status}`);
    const json = await res.json();
    storeInCache(`mixnodes-${status}`, JSON.stringify(json));
    return json;
  };

  static fetchMixnodeByID = async (id: string): Promise<MixNodeResponseItem | undefined> => {
    const response = await fetch(`${MIXNODE_API}/${id}`);

    // when the mixnode is not found, returned undefined
    if (response.status === 404) {
      return undefined;
    }

    return response.json();
  };

  static fetchGateways = async (): Promise<LocatedGateway[]> => {
    // const res = await fetch(GATEWAYS_API);
    // const gatewaysAnnotated: GatewayBondAnnotated[] = await res.json();
    // const res2 = await fetch(GATEWAYS_EXPLORER_API);
    // const locatedGateways: LocatedGateway[] = await res2.json();
    // const locatedGatewaysByOwner = keyBy(locatedGateways, 'owner');
    // return gatewaysAnnotated.map(({ gateway_bond, node_performance }) => ({
    //   ...gateway_bond,
    //   node_performance,
    //   location: locatedGatewaysByOwner[gateway_bond.owner]?.location,
    // }));

    const res = await fetch(LEGACY_GATEWAYS_API);
    const locatedGateways: LocatedGateway[] = await res.json();
    return locatedGateways;
  };

  static fetchGatewayUptimeStoryById = async (id: string): Promise<UptimeStoryResponse> =>
    (await fetch(`${UPTIME_STORY_API_GATEWAY}/${id}/history`)).json();

  static fetchGatewayReportById = async (id: string): Promise<GatewayReportResponse> =>
    (await fetch(`${UPTIME_STORY_API_GATEWAY}/${id}/report`)).json();

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

  static fetchDelegationsById = async (id: string): Promise<DelegationsResponse> =>
    (await fetch(`${MIXNODE_API}/${id}/delegations`)).json();

  static fetchUniqDelegationsById = async (id: string): Promise<UniqDelegationsResponse> =>
    (await fetch(`${MIXNODE_API}/${id}/delegations/summed`)).json();

  static fetchStatsById = async (id: string): Promise<StatsResponse> =>
    (await fetch(`${MIXNODE_API}/${id}/stats`)).json();

  static fetchMixnodeDescriptionById = async (id: string): Promise<MixNodeDescriptionResponse> =>
    (await fetch(`${MIXNODE_API}/${id}/description`)).json();

  static fetchMixnodeEconomicDynamicsStatsById = async (id: string): Promise<MixNodeEconomicDynamicsStatsResponse> =>
    (await fetch(`${MIXNODE_API}/${id}/economic-dynamics-stats`)).json();

  static fetchStatusById = async (id: string): Promise<StatusResponse> => (await fetch(`${MIXNODE_PING}/${id}`)).json();

  static fetchUptimeStoryById = async (id: string): Promise<UptimeStoryResponse> =>
    (await fetch(`${UPTIME_STORY_API}/${id}/history`)).json();

  static fetchServiceProviders = async (): Promise<DirectoryServiceProvider[]> => {
    const res = await fetch(SERVICE_PROVIDERS);
    const json = await res.json();
    return json;
  };

  static fetchNodes = async () => {
    const res = await fetch(TEMP_UNSTABLE_NYM_NODES);
    const json = await res.json();
    return json;
  }

  static fetchNodeById = async (id: number) => {
    const res = await fetch(`${TEMP_UNSTABLE_NYM_NODES}/${id}`);
    const json = await res.json();
    return json;
  }

  static fetchNymNodeUptimeHistoryById = async (id: number | string) => {
    const res = await fetch(`${NYM_API_NODE_UPTIME}/${id}`)
    const json = await res.json();
    return json;
  }

  static fetchNymNodePerformanceById = async (id: number | string) => {
    const res = await fetch(`${NYM_API_NODE_PERFORMANCE}/${id}`)
    const json = await res.json();
    return json;
  }

  static fetchAccountById = async (id: string) => {
    const res = await fetch(`${TEMP_UNSTABLE_ACCOUNT}/${id}`);
    const json = await res.json();
    return json;
  }
}

export const getEnvironment = (): Environment => {
  const matchEnv = (env: Environment) => API_BASE_URL?.toLocaleLowerCase().includes(env) && env;
  return matchEnv('sandbox') || matchEnv('qa') || 'mainnet';
};
