/*
 * Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

import { JsonObject } from '@cosmjs/cosmwasm-stargate/build/queries';
// eslint-disable-next-line import/no-cycle
import { INymdQuery } from './query-client';
import {
  ContractStateParams,
  Delegation,
  GatewayOwnershipResponse,
  LayerDistribution,
  MixnetContractVersion,
  MixOwnershipResponse,
  PagedAllDelegationsResponse,
  PagedDelegatorDelegationsResponse,
  PagedGatewayResponse,
  PagedMixDelegationsResponse,
  PagedMixnodeResponse,
  RewardingStatus,
} from './types';

interface SmartContractQuery {
  queryContractSmart(address: string, queryMsg: Record<string, unknown>): Promise<JsonObject>;
}

export default class NymdQuerier implements INymdQuery {
  client: SmartContractQuery;

  constructor(client: SmartContractQuery) {
    this.client = client;
  }

  getContractVersion(mixnetContractAddress: string): Promise<MixnetContractVersion> {
    return this.client.queryContractSmart(mixnetContractAddress, {
      get_contract_version: {},
    });
  }

  getMixNodesPaged(mixnetContractAddress: string, limit?: number, startAfter?: string): Promise<PagedMixnodeResponse> {
    return this.client.queryContractSmart(mixnetContractAddress, {
      get_mix_nodes: {
        limit,
        start_after: startAfter,
      },
    });
  }

  getGatewaysPaged(mixnetContractAddress: string, limit?: number, startAfter?: string): Promise<PagedGatewayResponse> {
    return this.client.queryContractSmart(mixnetContractAddress, {
      get_gateways: {
        limit,
        start_after: startAfter,
      },
    });
  }

  ownsMixNode(mixnetContractAddress: string, address: string): Promise<MixOwnershipResponse> {
    return this.client.queryContractSmart(mixnetContractAddress, {
      owns_mixnode: {
        address,
      },
    });
  }

  ownsGateway(mixnetContractAddress: string, address: string): Promise<GatewayOwnershipResponse> {
    return this.client.queryContractSmart(mixnetContractAddress, {
      owns_gateway: {
        address,
      },
    });
  }

  getStateParams(mixnetContractAddress: string): Promise<ContractStateParams> {
    return this.client.queryContractSmart(mixnetContractAddress, {
      state_params: {},
    });
  }

  getAllNetworkDelegationsPaged(
    mixnetContractAddress: string,
    limit?: number,
    startAfter?: [string, string],
  ): Promise<PagedAllDelegationsResponse> {
    return this.client.queryContractSmart(mixnetContractAddress, {
      get_all_network_delegations: {
        start_after: startAfter,
        limit,
      },
    });
  }

  getMixNodeDelegationsPaged(
    mixnetContractAddress: string,
    mixIdentity: string,
    limit?: number,
    startAfter?: string,
  ): Promise<PagedMixDelegationsResponse> {
    return this.client.queryContractSmart(mixnetContractAddress, {
      get_mixnode_delegations: {
        mix_identity: mixIdentity,
        start_after: startAfter,
        limit,
      },
    });
  }

  getDelegatorDelegationsPaged(
    mixnetContractAddress: string,
    delegator: string,
    limit?: number,
    startAfter?: string,
  ): Promise<PagedDelegatorDelegationsResponse> {
    return this.client.queryContractSmart(mixnetContractAddress, {
      get_delegator_delegations: {
        delegator,
        start_after: startAfter,
        limit,
      },
    });
  }

  getDelegationDetails(mixnetContractAddress: string, mixIdentity: string, delegator: string): Promise<Delegation> {
    return this.client.queryContractSmart(mixnetContractAddress, {
      get_delegation_details: {
        mix_identity: mixIdentity,
        delegator,
      },
    });
  }

  getLayerDistribution(mixnetContractAddress: string): Promise<LayerDistribution> {
    return this.client.queryContractSmart(mixnetContractAddress, {
      layer_distribution: {},
    });
  }

  getRewardPool(mixnetContractAddress: string): Promise<string> {
    return this.client.queryContractSmart(mixnetContractAddress, {
      get_reward_pool: {},
    });
  }

  getCirculatingSupply(mixnetContractAddress: string): Promise<string> {
    return this.client.queryContractSmart(mixnetContractAddress, {
      get_circulating_supply: {},
    });
  }

  getIntervalRewardPercent(mixnetContractAddress: string): Promise<number> {
    return this.client.queryContractSmart(mixnetContractAddress, {
      get_interval_reward_percent: {},
    });
  }

  getSybilResistancePercent(mixnetContractAddress: string): Promise<number> {
    return this.client.queryContractSmart(mixnetContractAddress, {
      get_sybil_resistance_percent: {},
    });
  }

  getRewardingStatus(
    mixnetContractAddress: string,
    mixIdentity: string,
    rewardingIntervalNonce: number,
  ): Promise<RewardingStatus> {
    return this.client.queryContractSmart(mixnetContractAddress, {
      get_rewarding_status: {
        mix_identity: mixIdentity,
        rewarding_interval_nonce: rewardingIntervalNonce,
      },
    });
  }
}
