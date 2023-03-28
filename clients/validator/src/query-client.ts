import { CosmWasmClient } from '@cosmjs/cosmwasm-stargate';
// eslint-disable-next-line import/no-cycle
import NyxdQuerier from './nyxd-querier';
import {
  Delegation,
  GatewayBond,
  GatewayOwnershipResponse,
  LayerDistribution,
  MixnetContractVersion,
  MixNodeDetails,
  MixOwnershipResponse,
  PagedAllDelegationsResponse,
  PagedDelegatorDelegationsResponse,
  PagedGatewayResponse,
  PagedMixDelegationsResponse,
  PagedMixNodeBondResponse,
  PagedMixNodeDetailsResponse,
  PagedUnbondedMixnodesResponse,
  StakeSaturationResponse,
  UnbondedMixnodeResponse,
  MixNodeBond,
  MixNodeRewarding,
} from '@nymproject/types';
import NymApiQuerier, { INymApiQuery } from './nym-api-querier';
import { ContractState, ICosmWasmQuery } from './types/shared';
import { RewardingParams } from '@nymproject/types';
import { Tendermint34Client } from '@cosmjs/tendermint-rpc';

export interface INyxdQuery {
  // nym-specific implemented inside NymQuerier
  getContractVersion(mixnetContractAddress: string): Promise<MixnetContractVersion>;
  getMixNodeBonds(
    mixnetContractAddress: string,
    limit?: number,
    startAfter?: string,
  ): Promise<PagedMixNodeBondResponse>;
  getMixNodesDetailed(
    mixnetContractAddress: string,
    limit?: number,
    startAfter?: string,
  ): Promise<PagedMixNodeDetailsResponse>;
  getGatewaysPaged(mixnetContractAddress: string, limit?: number, startAfter?: string): Promise<PagedGatewayResponse>;
  getOwnedMixnode(mixnetContractAddress: string, address: string): Promise<MixOwnershipResponse>;
  ownsGateway(mixnetContractAddress: string, address: string): Promise<GatewayOwnershipResponse>;
  getStateParams(mixnetContractAddress: string): Promise<ContractState>;
  getAllDelegationsPaged(
    mixnetContractAddress: string,
    limit?: number,
    startAfter?: [string, string],
  ): Promise<PagedAllDelegationsResponse>;
  getMixNodeDelegationsPaged(
    mixnetContractAddress: string,
    mix_id: number,
    limit?: number,
    startAfter?: string,
  ): Promise<PagedMixDelegationsResponse>;
  getDelegatorDelegationsPaged(
    mixnetContractAddress: string,
    delegator: string,
    limit?: number,
    startAfter?: string,
  ): Promise<PagedDelegatorDelegationsResponse>;
  getDelegationDetails(mixnetContractAddress: string, mix_id: number, delegator: string): Promise<Delegation>;
  getLayerDistribution(mixnetContractAddress: string): Promise<LayerDistribution>;
  getRewardParams(mixnetContractAddress: string): Promise<RewardingParams>;
  getStakeSaturation(mixnetContractAddress: string, mixId: number): Promise<StakeSaturationResponse>;
  getUnbondedMixNodeInformation(mixnetContractAddress: string, mixId: number): Promise<UnbondedMixnodeResponse>;
  getMixnodeRewardingDetails(mixnetContractAddress: string, mixId: number): Promise<MixNodeRewarding>;
}

export interface IQueryClient extends ICosmWasmQuery, INyxdQuery, INymApiQuery {}

export default class QueryClient extends CosmWasmClient implements IQueryClient {
  private nyxdQuerier: NyxdQuerier;

  private nymApiQuerier: NymApiQuerier;

  private constructor(tmClient: Tendermint34Client, nymApiUrl: string) {
    super(tmClient);
    this.nyxdQuerier = new NyxdQuerier(this);
    this.nymApiQuerier = new NymApiQuerier(nymApiUrl);
  }

  public static async connectWithNym(nyxdUrl: string, nymApiUrl: string): Promise<QueryClient> {
    const tmClient = await Tendermint34Client.connect(nyxdUrl);
    return new QueryClient(tmClient, nymApiUrl);
  }

  getContractVersion(mixnetContractAddress: string): Promise<MixnetContractVersion> {
    return this.nyxdQuerier.getContractVersion(mixnetContractAddress);
  }

  getMixNodeBonds(
    mixnetContractAddress: string,
    limit?: number,
    startAfter?: string,
  ): Promise<PagedMixNodeBondResponse> {
    return this.nyxdQuerier.getMixNodeBonds(mixnetContractAddress, limit, startAfter);
  }

  getMixNodesDetailed(
    mixnetContractAddress: string,
    limit?: number,
    startAfter?: string,
  ): Promise<PagedMixNodeDetailsResponse> {
    return this.nyxdQuerier.getMixNodesDetailed(mixnetContractAddress, limit, startAfter);
  }

  getStakeSaturation(mixnetContractAddress: string, mixId: number): Promise<StakeSaturationResponse> {
    return this.nyxdQuerier.getStakeSaturation(mixnetContractAddress, mixId);
  }

  getMixnodeRewardingDetails(mixnetContractAddress: string, mixId: number): Promise<MixNodeRewarding> {
    return this.nyxdQuerier.getMixnodeRewardingDetails(mixnetContractAddress, mixId);
  }

  getGatewaysPaged(mixnetContractAddress: string, limit?: number, startAfter?: string): Promise<PagedGatewayResponse> {
    return this.nyxdQuerier.getGatewaysPaged(mixnetContractAddress, limit, startAfter);
  }

  getOwnedMixnode(mixnetContractAddress: string, address: string): Promise<MixOwnershipResponse> {
    return this.nyxdQuerier.getOwnedMixnode(mixnetContractAddress, address);
  }

  ownsGateway(mixnetContractAddress: string, address: string): Promise<GatewayOwnershipResponse> {
    return this.nyxdQuerier.ownsGateway(mixnetContractAddress, address);
  }

  getUnbondedMixNodes(
    mixnetContractAddress: string,
    limit?: number,
    startAfter?: string,
  ): Promise<PagedUnbondedMixnodesResponse> {
    return this.nyxdQuerier.getUnbondedMixNodes(mixnetContractAddress, limit, startAfter);
  }

  getUnbondedMixNodeInformation(mixnetContractAddress: string, mixId: number): Promise<UnbondedMixnodeResponse> {
    return this.nyxdQuerier.getUnbondedMixNodeInformation(mixnetContractAddress, mixId);
  }

  getStateParams(mixnetContractAddress: string): Promise<ContractState> {
    return this.nyxdQuerier.getStateParams(mixnetContractAddress);
  }

  getAllDelegationsPaged(
    mixnetContractAddress: string,
    limit?: number,
    startAfter?: [string, string],
  ): Promise<PagedAllDelegationsResponse> {
    return this.nyxdQuerier.getAllDelegationsPaged(mixnetContractAddress, limit, startAfter);
  }

  getMixNodeDelegationsPaged(
    mixnetContractAddress: string,
    mix_id: number,
    limit?: number,
    startAfter?: string,
  ): Promise<PagedMixDelegationsResponse> {
    return this.nyxdQuerier.getMixNodeDelegationsPaged(mixnetContractAddress, mix_id, limit, startAfter);
  }

  getDelegatorDelegationsPaged(
    mixnetContractAddress: string,
    delegator: string,
    limit?: number,
    startAfter?: string,
  ): Promise<PagedDelegatorDelegationsResponse> {
    return this.nyxdQuerier.getDelegatorDelegationsPaged(mixnetContractAddress, delegator, limit, startAfter);
  }

  getDelegationDetails(mixnetContractAddress: string, mix_id: number, delegator: string): Promise<Delegation> {
    return this.nyxdQuerier.getDelegationDetails(mixnetContractAddress, mix_id, delegator);
  }

  getLayerDistribution(mixnetContractAddress: string): Promise<LayerDistribution> {
    return this.nyxdQuerier.getLayerDistribution(mixnetContractAddress);
  }

  getRewardParams(mixnetContractAddress: string): Promise<RewardingParams> {
    return this.nyxdQuerier.getRewardParams(mixnetContractAddress);
  }

  getCachedGateways(): Promise<GatewayBond[]> {
    return this.nymApiQuerier.getCachedGateways();
  }

  getCachedMixnodes(): Promise<MixNodeBond[]> {
    return this.nymApiQuerier.getCachedMixnodes();
  }

  getActiveMixnodes(): Promise<MixNodeDetails[]> {
    return this.nymApiQuerier.getActiveMixnodes();
  }

  getRewardedMixnodes(): Promise<MixNodeBond[]> {
    return this.nymApiQuerier.getRewardedMixnodes();
  }

  getSpendableCoins(vestingContractAddress: string, vestingAccountAddress: string): Promise<any> {
    return this.nyxdQuerier.getSpendableCoins(vestingContractAddress, vestingAccountAddress);
  }
}
