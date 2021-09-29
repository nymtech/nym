// import { Requester } from './requester';
export interface ClientConfig {
  //   requester?: Requester;
  url: string;
  version: string;
}

export enum Layer {
  One = 'One',
  Two = 'Two',
  Three = 'Three',
}
export interface MixNode {
  host: string;
  location: string;
  sphinx_key: string;
  identity_key: string;
  version: string;
  mix_port: number;
  verloc_port: number;
  http_api_port: number;
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

export interface MixNodeResponseItem {
  bond_amount: Amount;
  total_delegation: Amount;
  owner: string;
  layer: Layer;
  location: {
    country_name: string
    lat: number
    lng: number
    three_letter_iso_country_code: string
    two_letter_iso_country_code: string
  }
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
  bond_amount: Amount;
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
  address: string
  proposer_priority: string
  pub_key: {
    type: string
    value: string
  }
}
export interface ValidatorsResponse {
  block_height: number
  count: string
  total: string
  validators: Validator[]
}

export type CountryData = {
  ISO3: string
  nodes: number
}

export type Delegation = {
  owner: string,
  amount: Amount,
  block_height: number
}

export type DelegationsResponse = Delegation[];

export type CountryDataResponse = CountryData[];

export type BlockType = {
  
}
export type BlockResponse = BlockType;

export interface ApiState<RESPONSE> {
  isLoading: boolean;
  data?: RESPONSE;
  error?: Error;
}