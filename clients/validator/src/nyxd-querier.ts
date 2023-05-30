/*
 * Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */
// eslint-disable-next-line import/no-cycle
import { INyxdQuery } from './query-client';
import {
  Delegation, OriginalVestingResponse, RewardingParams, StakeSaturationResponse, VestingAccountInfo,
  UnbondedMixnodeResponse,
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
  LayerDistribution,
  ContractState, VestingAccountsCoinPaged, VestingAccountsPaged, DelegationTimes, Delegations, Period, VestingAccountNode, DelegationBlock
} from '@nymproject/types';
import { SmartContractQuery } from './types/shared';
import { Coin } from 'cosmjs-types/cosmos/base/v1beta1/coin';

export default class NyxdQuerier implements INyxdQuery {
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
      get_owned_gateway: {
        address,
      },
    });
  }

  getStateParams(mixnetContractAddress: string): Promise<ContractState> {
    return this.client.queryContractSmart(mixnetContractAddress, {
      get_state: {},
    });
  }

  getAllDelegationsPaged(
    mixnetContractAddress: string,
    limit?: number,
    startAfter?: [string, string],
  ): Promise<PagedAllDelegationsResponse> {
    return this.client.queryContractSmart(mixnetContractAddress, {
      get_all_delegations: {
        start_after: startAfter,
        limit,
      },
    });
  }

  getMixNodeDelegationsPaged(
    mixnetContractAddress: string,
    mix_id: number,
    limit?: number,
    startAfter?: string,
  ): Promise<PagedMixDelegationsResponse> {
    return this.client.queryContractSmart(mixnetContractAddress, {
      get_mixnode_delegations: {
        mix_id: mix_id,
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

  getDelegationDetails(mixnetContractAddress: string, mix_id: number, delegator: string): Promise<Delegation> {
    return this.client.queryContractSmart(mixnetContractAddress, {
      get_delegation_details: {
        mix_id: mix_id,
        delegator,
      },
    });
  }

  getLayerDistribution(mixnetContractAddress: string): Promise<LayerDistribution> {
    return this.client.queryContractSmart(mixnetContractAddress, {
      get_layer_distribution: {},
    });
  }

  getRewardParams(mixnetContractAddress: string): Promise<RewardingParams> {
    return this.client.queryContractSmart(mixnetContractAddress, {
      get_rewarding_params: {},
    });
  }

  getSpendableCoins(vestingContractAddress: string, vestingAccountAddress: string): Promise<any> {
    return this.client.queryContractSmart(vestingContractAddress, {
      vesting_account_address: vestingAccountAddress,
    });
  }

  getVestingAccountsPaged(vestingContractAddress: string): Promise<VestingAccountsPaged> {
    return this.client.queryContractSmart(vestingContractAddress, {
      get_accounts_paged: {}
    });
  }

  getVestingAmountsAccountsPaged(vestingContractAddress: string): Promise<VestingAccountsCoinPaged> {
    return this.client.queryContractSmart(vestingContractAddress, {
      get_accounts_vesting_coins_paged: {}
    });
  }

  getLockedTokens(vestingContractAddress: string, vestingAccountAddress: string): Promise<Coin> {
    return this.client.queryContractSmart(vestingContractAddress, {
      locked_coins: { vesting_account_address: vestingAccountAddress }
    });
  }

  getSpendableTokens(vestingContractAddress: string, vestingAccountAddress: string): Promise<Coin> {
    return this.client.queryContractSmart(vestingContractAddress, {
      spendable_coins: { vesting_account_address: vestingAccountAddress }
    });
  }

  getVestedTokens(vestingContractAddress: string, vestingAccountAddress: string): Promise<Coin> {
    return this.client.queryContractSmart(vestingContractAddress, {
      get_vested_coins: { vesting_account_address: vestingAccountAddress }
    });
  }

  getVestingTokens(vestingContractAddress: string, vestingAccountAddress: string): Promise<Coin> {
    return this.client.queryContractSmart(vestingContractAddress, {
      get_vesting_coins: { vesting_account_address: vestingAccountAddress }
    });
  }

  getSpendableVestedTokens(vestingContractAddress: string, vestingAccountAddress: string): Promise<Coin> {
    return this.client.queryContractSmart(vestingContractAddress, {
      get_spendable_vested_coins: { vesting_account_address: vestingAccountAddress }
    });
  }

  getSpendableRewards(vestingContractAddress: string, vestingAccountAddress: string): Promise<Coin> {
    return this.client.queryContractSmart(vestingContractAddress, {
      get_spendable_reward_coins: { vesting_account_address: vestingAccountAddress }
    });
  }

  getDelegatedCoins(vestingContractAddress: string, vestingAccountAddress: string): Promise<Coin> {
    return this.client.queryContractSmart(vestingContractAddress, {
      get_delegated_coins: { vesting_account_address: vestingAccountAddress }
    });
  }

  getPledgedCoins(vestingContractAddress: string, vestingAccountAddress: string): Promise<Coin> {
    return this.client.queryContractSmart(vestingContractAddress, {
      get_pledged_coins: { vesting_account_address: vestingAccountAddress }
    });
  }

  getStakedCoins(vestingContractAddress: string, vestingAccountAddress: string): Promise<Coin> {
    return this.client.queryContractSmart(vestingContractAddress, {
      get_staked_coins: { vesting_account_address: vestingAccountAddress }
    });
  }

  getWithdrawnCoins(vestingContractAddress: string, vestingAccountAddress: string): Promise<Coin> {
    return this.client.queryContractSmart(vestingContractAddress, {
      get_withdrawn_coins: { vesting_account_address: vestingAccountAddress }
    });
  }

  getStartTime(vestingContractAddress: string, vestingAccountAddress: string): Promise<string> {
    return this.client.queryContractSmart(vestingContractAddress, {
      get_start_time: { vesting_account_address: vestingAccountAddress }
    });
  }

  getEndTime(vestingContractAddress: string, vestingAccountAddress: string): Promise<string> {
    return this.client.queryContractSmart(vestingContractAddress, {
      get_end_time: { vesting_account_address: vestingAccountAddress }
    });
  }

  getOriginalVestingDetails(vestingContractAddress: string, vestingAccountAddress: string): Promise<OriginalVestingResponse> {
    return this.client.queryContractSmart(vestingContractAddress, {
      get_original_vesting: { vesting_account_address: vestingAccountAddress }
    });
  }

  getHistoricStakingRewards(vestingContractAddress: string, vestingAccountAddress: string): Promise<Coin> {
    return this.client.queryContractSmart(vestingContractAddress, {
      get_historical_vesting_staking_reward: { vesting_account_address: vestingAccountAddress }
    });
  }

  getAccountDetails(vestingContractAddress: string, address: string): Promise<VestingAccountInfo> {
    return this.client.queryContractSmart(vestingContractAddress, {
      get_account: { address: address }
    });
  }

  getMixnode(vestingContractAddress: string, address: string): Promise<VestingAccountNode> {
    return this.client.queryContractSmart(vestingContractAddress, {
      get_mixnode: { address: address }
    });
  }

  getGateway(vestingContractAddress: string, address: string): Promise<VestingAccountNode> {
    return this.client.queryContractSmart(vestingContractAddress, {
      get_gateway: { address: address }
    });
  }

  getDelegationTimes(vestingContractAddress: string, mix_id: number, address: string): Promise<DelegationTimes> {
    return this.client.queryContractSmart(vestingContractAddress, {
      get_delegation_times: { mix_id: mix_id, address: address }
    });
  }

  getAllDelegations(vestingContractAddress: string): Promise<Delegations> {
    return this.client.queryContractSmart(vestingContractAddress, {
      get_all_delegations: {}
    });
  }

  getDelegation(vestingContractAddress: string, address: string, mix_id: number): Promise<DelegationBlock> {
    return this.client.queryContractSmart(vestingContractAddress, {
      get_all_delegations: {address: address, mix_id: mix_id}
    });
  }

  getTotalDelegationAmount(vestingContractAddress: string, address: string, mix_id: number, block_timestamp_sec: number): Promise<Coin> {
    return this.client.queryContractSmart(vestingContractAddress, {
      get_all_delegations: {address: address, mix_id: mix_id, block_timestamp_sec: block_timestamp_sec}
    });
  }

  getCurrentVestingPeriod(vestingContractAddress: string, address: string): Promise<Period> {
    return this.client.queryContractSmart(vestingContractAddress, {
      get_current_vesting_period: { address: address }
    });
  }
}
