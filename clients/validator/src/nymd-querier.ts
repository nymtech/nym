/*
 * Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

import {
  ContractStateParams,
  Delegation,
  GatewayOwnershipResponse,
  MixnetContractVersion,
  MixOwnershipResponse,
  PagedAllDelegationsResponse,
  PagedDelegatorDelegationsResponse,
  PagedGatewayResponse,
  PagedMixDelegationsResponse,
  PagedMixNodeBondResponse,
  PagedMixNodeDetailsResponse,
  PagedUnbondedMixnodesResponse,
  RewardingStatus,
  StakeSaturationResponse,
  UnbondedMixnodeResponse,
  LayerDistribution,
} from '../compiledTypes';
import { INymdQuery, SmartContractQuery } from './types';

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
  getMixNodeBonds(
    mixnetContractAddress: string,
    limit?: number,
    startAfter?: string,
  ): Promise<PagedMixNodeBondResponse> {
    return this.client.queryContractSmart(mixnetContractAddress, {
      get_mix_node_bonds: {
        limit,
        start_after: startAfter,
      },
    });
  }

  getMixNodesDetailed(
    mixnetContractAddress: string,
    limit?: number,
    startAfter?: string,
  ): Promise<PagedMixNodeDetailsResponse> {
    return this.client.queryContractSmart(mixnetContractAddress, {
      get_mix_nodes_detailed: {
        limit,
        start_after: startAfter,
      },
    });
  }

  getStakeSaturation(mixnetContractAddress: string, mixId: number): Promise<StakeSaturationResponse> {
    return this.client.queryContractSmart(mixnetContractAddress, {
      get_stake_saturation: { mix_id: mixId },
    });
  }

  getMixnodeRewardingDetails(mixnetContractAddress: string, mixId: number): Promise<any> {
    return this.client.queryContractSmart(mixnetContractAddress, {
      get_mixnode_rewarding_details: { mix_id: mixId },
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

  getOwnedMixnode(mixnetContractAddress: string, address: string): Promise<MixOwnershipResponse> {
    return this.client.queryContractSmart(mixnetContractAddress, {
      get_owned_mixnode: {
        address,
      },
    });
  }

  getUnbondedMixNodes(
    mixnetContractAddress: string,
    limit?: number,
    startAfter?: string,
  ): Promise<PagedUnbondedMixnodesResponse> {
    return this.client.queryContractSmart(mixnetContractAddress, {
      get_unbonded_mix_nodes: { limit, start_after: startAfter },
    });
  }

  getUnbondedMixNodeInformation(mixnetContractAddress: string, mixId: number): Promise<UnbondedMixnodeResponse> {
    return this.client.queryContractSmart(mixnetContractAddress, {
      get_unbonded_mix_node_information: { mix_id: mixId },
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

  getSpendableCoins(vestingContractAddress: string, vestingAccountAddress: string): Promise<any> {
    return this.client.queryContractSmart(vestingContractAddress, {
      vesting_account_address: vestingAccountAddress,
    });
  }
}
