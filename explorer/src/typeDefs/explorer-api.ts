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
  profit_margin_percent: number;
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

export const toMixnodeStatus = (
  status?: MixnodeStatusWithAll,
): MixnodeStatus | undefined => {
  if (!status || status === MixnodeStatusWithAll.all) {
    return undefined;
  }
  return status as unknown as MixnodeStatus;
};

export interface MixNodeResponseItem {
  pledge_amount: Amount;
  total_delegation: Amount;
  owner: string;
  layer: string;
  status: MixnodeStatus;
  location: {
    country_name: string;
    lat: number;
    lng: number;
    three_letter_iso_country_code: string;
    two_letter_iso_country_code: string;
  };
  mix_node: MixNode;
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

export type MixNodeHistoryResponse = StatsResponse;

export interface GatewayResponseItem {
  block_height: number;
  pledge_amount: Amount;
  total_delegation: Amount;
  owner: string;
  gateway: Gateway;
}

export type GatewayResponse = GatewayResponseItem[];

export interface GatewayReportResponse {
  identity: string;
  owner: string;
  most_recent_ipv4: boolean;
  most_recent_ipv6: boolean;
  last_hour_ipv4: number;
  last_hour_ipv6: number;
  last_day_ipv4: number;
  last_day_ipv6: number;
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

export type DelegationsResponse = Delegation[];

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
