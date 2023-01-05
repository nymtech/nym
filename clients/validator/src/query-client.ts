import { CosmWasmClient, JsonObject } from '@cosmjs/cosmwasm-stargate';
import { Tendermint34Client } from '@cosmjs/tendermint-rpc';
import {
  Account,
  Block,
  Coin,
  DeliverTxResponse,
  IndexedTx,
  SearchTxFilter,
  SearchTxQuery,
  SequenceResponse,
} from '@cosmjs/stargate';
import { Code, CodeDetails, Contract, ContractCodeHistoryEntry } from '@cosmjs/cosmwasm-stargate/build/cosmwasmclient';
// eslint-disable-next-line import/no-cycle
import NyxdQuerier from './nyxd-querier';
import {
  ContractStateParams,
  Delegation,
  GatewayBond,
  GatewayOwnershipResponse,
  LayerDistribution,
  MixnetContractVersion,
  MixNodeBond,
  MixOwnershipResponse,
  PagedAllDelegationsResponse,
  PagedDelegatorDelegationsResponse,
  PagedGatewayResponse,
  PagedMixDelegationsResponse,
  PagedMixnodeResponse,
  RewardingStatus,
} from './types';
import NymApiQuerier, { INymApiQuery as INymApiQuery } from './nym-api-querier';

export interface ICosmWasmQuery {
  // methods exposed by `CosmWasmClient`
  getChainId(): Promise<string>;
  getHeight(): Promise<number>;
  getAccount(searchAddress: string): Promise<Account | null>;
  getSequence(address: string): Promise<SequenceResponse>;
  getBlock(height?: number): Promise<Block>;
  getBalance(address: string, searchDenom: string): Promise<Coin>;
  getTx(id: string): Promise<IndexedTx | null>;
  searchTx(query: SearchTxQuery, filter?: SearchTxFilter): Promise<readonly IndexedTx[]>;
  disconnect(): void;
  broadcastTx(tx: Uint8Array, timeoutMs?: number, pollIntervalMs?: number): Promise<DeliverTxResponse>;
  getCodes(): Promise<readonly Code[]>;
  getCodeDetails(codeId: number): Promise<CodeDetails>;
  getContracts(codeId: number): Promise<readonly string[]>;
  getContract(address: string): Promise<Contract>;
  getContractCodeHistory(address: string): Promise<readonly ContractCodeHistoryEntry[]>;
  queryContractRaw(address: string, key: Uint8Array): Promise<Uint8Array | null>;
  queryContractSmart(address: string, queryMsg: Record<string, unknown>): Promise<JsonObject>;
}

export interface INyxdQuery {
  // nym-specific implemented inside NymQuerier
  getContractVersion(mixnetContractAddress: string): Promise<MixnetContractVersion>;

  getMixNodesPaged(mixnetContractAddress: string, limit?: number, startAfter?: string): Promise<PagedMixnodeResponse>;
  getGatewaysPaged(mixnetContractAddress: string, limit?: number, startAfter?: string): Promise<PagedGatewayResponse>;
  ownsMixNode(mixnetContractAddress: string, address: string): Promise<MixOwnershipResponse>;
  ownsGateway(mixnetContractAddress: string, address: string): Promise<GatewayOwnershipResponse>;
  getStateParams(mixnetContractAddress: string): Promise<ContractStateParams>;

  getAllNetworkDelegationsPaged(
    mixnetContractAddress: string,
    limit?: number,
    startAfter?: [string, string],
  ): Promise<PagedAllDelegationsResponse>;
  getMixNodeDelegationsPaged(
    mixnetContractAddress: string,
    mixIdentity: string,
    limit?: number,
    startAfter?: string,
  ): Promise<PagedMixDelegationsResponse>;
  getDelegatorDelegationsPaged(
    mixnetContractAddress: string,
    delegator: string,
    limit?: number,
    startAfter?: string,
  ): Promise<PagedDelegatorDelegationsResponse>;
  getDelegationDetails(mixnetContractAddress: string, mixIdentity: string, delegator: string): Promise<Delegation>;

  getLayerDistribution(mixnetContractAddress: string): Promise<LayerDistribution>;
  getRewardPool(mixnetContractAddress: string): Promise<string>;
  getCirculatingSupply(mixnetContractAddress: string): Promise<string>;
  getIntervalRewardPercent(mixnetContractAddress: string): Promise<number>;
  getSybilResistancePercent(mixnetContractAddress: string): Promise<number>;
  getRewardingStatus(
    mixnetContractAddress: string,
    mixIdentity: string,
    rewardingIntervalNonce: number,
  ): Promise<RewardingStatus>;
}

export interface IQueryClient extends ICosmWasmQuery, INyxdQuery, INymApiQuery { }

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

  getMixNodesPaged(mixnetContractAddress: string, limit?: number, startAfter?: string): Promise<PagedMixnodeResponse> {
    return this.nyxdQuerier.getMixNodesPaged(mixnetContractAddress, limit, startAfter);
  }

  getGatewaysPaged(mixnetContractAddress: string, limit?: number, startAfter?: string): Promise<PagedGatewayResponse> {
    return this.nyxdQuerier.getGatewaysPaged(mixnetContractAddress, limit, startAfter);
  }

  ownsMixNode(mixnetContractAddress: string, address: string): Promise<MixOwnershipResponse> {
    return this.nyxdQuerier.ownsMixNode(mixnetContractAddress, address);
  }

  ownsGateway(mixnetContractAddress: string, address: string): Promise<GatewayOwnershipResponse> {
    return this.nyxdQuerier.ownsGateway(mixnetContractAddress, address);
  }

  getStateParams(mixnetContractAddress: string): Promise<ContractStateParams> {
    return this.nyxdQuerier.getStateParams(mixnetContractAddress);
  }

  getAllNetworkDelegationsPaged(
    mixnetContractAddress: string,
    limit?: number,
    startAfter?: [string, string],
  ): Promise<PagedAllDelegationsResponse> {
    return this.nyxdQuerier.getAllNetworkDelegationsPaged(mixnetContractAddress, limit, startAfter);
  }

  getMixNodeDelegationsPaged(
    mixnetContractAddress: string,
    mixIdentity: string,
    limit?: number,
    startAfter?: string,
  ): Promise<PagedMixDelegationsResponse> {
    return this.nyxdQuerier.getMixNodeDelegationsPaged(mixnetContractAddress, mixIdentity, limit, startAfter);
  }

  getDelegatorDelegationsPaged(
    mixnetContractAddress: string,
    delegator: string,
    limit?: number,
    startAfter?: string,
  ): Promise<PagedDelegatorDelegationsResponse> {
    return this.nyxdQuerier.getDelegatorDelegationsPaged(mixnetContractAddress, delegator, limit, startAfter);
  }

  getDelegationDetails(mixnetContractAddress: string, mixIdentity: string, delegator: string): Promise<Delegation> {
    return this.nyxdQuerier.getDelegationDetails(mixnetContractAddress, mixIdentity, delegator);
  }

  getLayerDistribution(mixnetContractAddress: string): Promise<LayerDistribution> {
    return this.nyxdQuerier.getLayerDistribution(mixnetContractAddress);
  }

  getRewardPool(mixnetContractAddress: string): Promise<string> {
    return this.nyxdQuerier.getRewardPool(mixnetContractAddress);
  }

  getCirculatingSupply(mixnetContractAddress: string): Promise<string> {
    return this.nyxdQuerier.getCirculatingSupply(mixnetContractAddress);
  }

  getIntervalRewardPercent(mixnetContractAddress: string): Promise<number> {
    return this.nyxdQuerier.getIntervalRewardPercent(mixnetContractAddress);
  }

  getSybilResistancePercent(mixnetContractAddress: string): Promise<number> {
    return this.nyxdQuerier.getSybilResistancePercent(mixnetContractAddress);
  }

  getRewardingStatus(
    mixnetContractAddress: string,
    mixIdentity: string,
    rewardingIntervalNonce: number,
  ): Promise<RewardingStatus> {
    return this.nyxdQuerier.getRewardingStatus(mixnetContractAddress, mixIdentity, rewardingIntervalNonce);
  }

  getCachedGateways(): Promise<GatewayBond[]> {
    return this.nymApiQuerier.getCachedGateways();
  }

  getCachedMixnodes(): Promise<MixNodeBond[]> {
    return this.nymApiQuerier.getCachedMixnodes();
  }

  getActiveMixnodes(): Promise<MixNodeBond[]> {
    return this.nymApiQuerier.getActiveMixnodes();
  }

  getRewardedMixnodes(): Promise<MixNodeBond[]> {
    return this.nymApiQuerier.getRewardedMixnodes();
  }
}
