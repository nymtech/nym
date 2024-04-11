/* eslint-disable camelcase */

export interface ClientConfig {
  url: string;
  version: string;
}

export interface SummaryOverviewResponse {
  mixnodes: {
    count: number;
    activeset: {
      active: number;
      standby: number;
      inactive: number;
    };
  };
  gateways: {
    count: number;
  };
  validators: {
    count: number;
  };
}

export interface MixNode {
  host: string;
  mix_port: number;
  http_api_port: number;
  verloc_port: number;
  sphinx_key: string;
  identity_key: string;
  version: string;
  location: string;
}

export interface Gateway {
  host: string;
  mix_port: number;
  clients_port: number;
  location: string;
  sphinx_key: string;
  identity_key: string;
  version: string;
}

export interface Amount {
  denom: string;
  amount: number;
}

export enum MixnodeStatus {
  active = 'active', // in both the active set and the rewarded set
  standby = 'standby', // only in the rewarded set
  inactive = 'inactive', // in neither the rewarded set nor the active set
}

export enum MixnodeStatusWithAll {
  active = 'active', // in both the active set and the rewarded set
  standby = 'standby', // only in the rewarded set
  inactive = 'inactive', // in neither the rewarded set nor the active set
  all = 'all', // any status
}

export type MixnodeStatusWithAllString = 'active' | 'standby' | 'inactive' | 'all';

export const toMixnodeStatus = (status?: MixnodeStatusWithAllString): MixnodeStatus | undefined => {
  if (!status || status === MixnodeStatusWithAll.all) {
    return undefined;
  }
  return status as unknown as MixnodeStatus;
};

export interface MixNodeResponseItem {
  mix_id: number;
  pledge_amount: Amount;
  total_delegation: Amount;
  owner: string;
  layer: string;
  status: MixnodeStatus;
  location: {
    country_name: string;
    latitude?: number;
    longitude?: number;
    three_letter_iso_country_code: string;
    two_letter_iso_country_code: string;
  };
  mix_node: MixNode;
  avg_uptime: number;
  node_performance: NodePerformance;
  stake_saturation: number;
  uncapped_saturation: number;
  operating_cost: Amount;
  profit_margin_percent: string;
  blacklisted: boolean;
}

export type MixNodeResponse = MixNodeResponseItem[];

export interface MixNodeReportResponse {
  identity: string;
  owner: string;
  most_recent_ipv4: boolean;
  most_recent_ipv6: boolean;
  last_hour_ipv4: number;
  last_hour_ipv6: number;
  last_day_ipv4: number;
  last_day_ipv6: number;
}

export interface StatsResponse {
  update_time: Date;
  previous_update_time: Date;
  packets_received_since_startup: number;
  packets_sent_since_startup: number;
  packets_explicitly_dropped_since_startup: number;
  packets_received_since_last_update: number;
  packets_sent_since_last_update: number;
  packets_explicitly_dropped_since_last_update: number;
}

export interface NodePerformance {
  most_recent: string;
  last_hour: string;
  last_24h: string;
}

export type MixNodeHistoryResponse = StatsResponse;

export interface GatewayBond {
  block_height: number;
  pledge_amount: Amount;
  total_delegation: Amount;
  owner: string;
  gateway: Gateway;
  node_performance: NodePerformance;
  location?: Location;
}

export interface GatewayBondAnnotated {
  gateway_bond: GatewayBond;
  node_performance: NodePerformance;
}

export interface Location {
  two_letter_iso_country_code: string;
  three_letter_iso_country_code: string;
  country_name: string;
  latitude?: number;
  longitude?: number;
}

export interface LocatedGateway {
  pledge_amount: Amount;
  owner: string;
  block_height: number;
  gateway: Gateway;
  proxy?: string;
  location?: Location;
}

export type GatewayResponse = GatewayBond[];

export interface GatewayReportResponse {
  identity: string;
  owner: string;
  most_recent: number;
  last_hour: number;
  last_day: number;
}

export type GatewayHistoryResponse = StatsResponse;

export interface MixNodeDescriptionResponse {
  name: string;
  description: string;
  link: string;
  location: string;
}

export type MixNodeStatsResponse = StatsResponse;

export interface Validator {
  address: string;
  proposer_priority: string;
  pub_key: {
    type: string;
    value: string;
  };
}
export interface ValidatorsResponse {
  block_height: number;
  count: string;
  total: string;
  validators: Validator[];
}

export type CountryData = {
  ISO3: string;
  nodes: number;
};

export type Delegation = {
  owner: string;
  amount: Amount;
  block_height: number;
};

export type DelegationUniq = {
  owner: string;
  amount: Amount;
};

export type DelegationsResponse = Delegation[];

export type UniqDelegationsResponse = DelegationUniq[];

export interface CountryDataResponse {
  [threeLetterCountryCode: string]: CountryData;
}

export type BlockType = number;
export type BlockResponse = BlockType;

export interface ApiState<RESPONSE> {
  isLoading: boolean;
  data?: RESPONSE;
  error?: Error;
}

export type StatusResponse = {
  pending: boolean;
  ports: {
    1789: boolean;
    1790: boolean;
    8000: boolean;
  };
};

export type UptimeTime = {
  date: string;
  uptime: number;
};

export type UptimeStoryResponse = {
  history: UptimeTime[];
  identity: string;
  owner: string;
};

export type MixNodeEconomicDynamicsStatsResponse = {
  stake_saturation: number;
  uncapped_saturation: number;
  // TODO: when v2 will be deployed, remove cases: VeryHigh, Moderate and VeryLow
  active_set_inclusion_probability: 'High' | 'Good' | 'Low';
  reserve_set_inclusion_probability: 'High' | 'Good' | 'Low';
  estimated_total_node_reward: number;
  estimated_operator_reward: number;
  estimated_delegators_reward: number;
  current_interval_uptime: number;
};

export type Environment = 'mainnet' | 'sandbox' | 'qa';

export type ServiceProviderType = 'Network Requester';

export type DirectoryServiceProvider = {
  id: string;
  description: string;
  address: string;
  gateway: string;
  routing_score: string | null;
  service_type: ServiceProviderType;
};

export type DirectoryService = {
  id: string;
  description: string;
  items: DirectoryServiceProvider[];
};
