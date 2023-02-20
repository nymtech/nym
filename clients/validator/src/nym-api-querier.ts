/*
 * Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */
import axios from 'axios';
import { GatewayBond, MixNodeBond, MixNodeDetails } from '@nymproject/types';

export const NYM_API_VERSION = '/v1';
export const NYM_API_GATEWAYS_PATH = `${NYM_API_VERSION}/gateways`;
export const NYM_API_MIXNODES_PATH = `${NYM_API_VERSION}/mixnodes`;
export const NYM_API_ACTIVE_MIXNODES_PATH = `${NYM_API_VERSION}/mixnodes/active`;
export const NYM_API_REWARDED_MIXNODES_PATH = `${NYM_API_VERSION}/mixnodes/rewarded`;

export interface INymApiQuery {
  getCachedMixnodes(): Promise<MixNodeBond[]>;

  getCachedGateways(): Promise<GatewayBond[]>;

  getActiveMixnodes(): Promise<MixNodeDetails[]>;

  getRewardedMixnodes(): Promise<MixNodeBond[]>;
}

export default class NymApiQuerier implements INymApiQuery {
  nymApiUrl: string;

  constructor(nymApiUrl: string) {
    this.nymApiUrl = nymApiUrl;
  }

  async getCachedMixnodes(): Promise<MixNodeBond[]> {
    const url = new URL(this.nymApiUrl);
    url.pathname += NYM_API_MIXNODES_PATH;

    const response = await axios.get(url.toString());
    if (response.status === 200) {
      return response.data;
    }
    throw new Error('None of the provided validator APIs seem to be alive');
  }

  async getCachedGateways(): Promise<GatewayBond[]> {
    const url = new URL(this.nymApiUrl);
    url.pathname += NYM_API_GATEWAYS_PATH;

    const response = await axios.get(url.toString());
    if (response.status === 200) {
      return response.data;
    }
    throw new Error('None of the provided validator APIs seem to be alive');
  }

  async getActiveMixnodes(): Promise<MixNodeDetails[]> {
    const url = new URL(this.nymApiUrl);
    url.pathname += NYM_API_ACTIVE_MIXNODES_PATH;

    const response = await axios.get(url.toString());
    if (response.status === 200) {
      return response.data;
    }
    throw new Error('None of the provided validator APIs seem to be alive');
  }

  async getRewardedMixnodes(): Promise<MixNodeBond[]> {
    const url = new URL(this.nymApiUrl);
    url.pathname += NYM_API_REWARDED_MIXNODES_PATH;

    const response = await axios.get(url.toString());
    if (response.status === 200) {
      return response.data;
    }
    throw new Error('None of the provided validator APIs seem to be alive');
  }
}
