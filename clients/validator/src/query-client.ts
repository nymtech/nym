import {CosmWasmClient} from "@cosmjs/cosmwasm-stargate";
import {
    ContractStateParams,
    Delegation,
    GatewayOwnershipResponse, LayerDistribution, MixnetContractVersion,
    MixOwnershipResponse, PagedAllDelegationsResponse, PagedDelegatorDelegationsResponse,
    PagedGatewayResponse,
    PagedMixDelegationsResponse,
    PagedMixnodeResponse, RewardingIntervalResponse, RewardingStatus
} from "./types";
import {Tendermint34Client} from "@cosmjs/tendermint-rpc";
import NymdQuerier from "./nymd-querier";
import {
    Account,
    Block,
    Coin,
    DeliverTxResponse,
    IndexedTx,
    SearchTxFilter,
    SearchTxQuery,
    SequenceResponse
} from "@cosmjs/stargate";
import {JsonObject} from "@cosmjs/cosmwasm-stargate/build/queries";
import {Code, CodeDetails, Contract, ContractCodeHistoryEntry} from "@cosmjs/cosmwasm-stargate/build/cosmwasmclient";
import {IValidatorApiQuery} from "./validator-api-querier";

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

export interface INymdQuery {
    // nym-specific implemented inside NymQuerier
    getContractVersion(mixnetContractAddress: string): Promise<MixnetContractVersion>;

    getMixNodesPaged(mixnetContractAddress: string, limit?: number, startAfter?: string): Promise<PagedMixnodeResponse>;
    getGatewaysPaged(mixnetContractAddress: string, limit?: number, startAfter?: string): Promise<PagedGatewayResponse>;
    ownsMixNode(mixnetContractAddress: string, address: string): Promise<MixOwnershipResponse>;
    ownsGateway(mixnetContractAddress: string, address: string): Promise<GatewayOwnershipResponse>;
    getStateParams(mixnetContractAddress: string): Promise<ContractStateParams>;
    getCurrentRewardingInterval(mixnetContractAddress: string): Promise<RewardingIntervalResponse>;

    getAllNetworkDelegationsPaged(mixnetContractAddress: string, limit?: number, startAfter?: [string, string]): Promise<PagedAllDelegationsResponse>;
    getMixNodeDelegationsPaged(mixnetContractAddress: string, mixIdentity: string, limit?: number, startAfter?: string): Promise<PagedMixDelegationsResponse>
    getDelegatorDelegationsPaged(mixnetContractAddress: string, delegator: string, limit?: number, startAfter?: string): Promise<PagedDelegatorDelegationsResponse>
    getDelegationDetails(mixnetContractAddress: string, mixIdentity: string, delegator: string): Promise<Delegation>;

    getLayerDistribution(mixnetContractAddress: string): Promise<LayerDistribution>;
    getRewardPool(mixnetContractAddress: string): Promise<string>;
    getCirculatingSupply(mixnetContractAddress: string): Promise<string>;
    getEpochRewardPercent(mixnetContractAddress: string): Promise<number>;
    getSybilResistancePercent(mixnetContractAddress: string): Promise<number>;
    getRewardingStatus(mixnetContractAddress: string, mixIdentity: string, rewardingIntervalNonce: number): Promise<RewardingStatus>;
}

export interface IQueryClient extends ICosmWasmQuery, INymdQuery, IValidatorApiQuery {}

/**
 * Takes care of network communication between this code and the validator.
 * Depends on `SigningCosmWasClient`, which signs all requests using keypairs
 * derived from on bech32 mnemonics.
 *
 * Wraps several methods from CosmWasmSigningClient so we can mock them for
 * unit testing.
 */
export default class QueryClient extends CosmWasmClient implements IQueryClient {
    private querier: NymdQuerier;
    validatorApiUrl: string;

    private constructor(tmClient: Tendermint34Client, validatorApiUrl: string,) {
        super(tmClient)
        this.querier = new NymdQuerier(this)
        this.validatorApiUrl = validatorApiUrl
    }


    public static async connectWithNym(nymdUrl: string, validatorApiUrl: string,): Promise<QueryClient> {
        const tmClient = await Tendermint34Client.connect(nymdUrl);
        return new QueryClient(tmClient, validatorApiUrl)
    }

    getContractVersion(mixnetContractAddress: string): Promise<MixnetContractVersion> {
        return this.querier.getContractVersion(mixnetContractAddress)
    }

    getMixNodesPaged(mixnetContractAddress: string, limit?: number, startAfter?: string): Promise<PagedMixnodeResponse> {
        return this.querier.getMixNodesPaged(mixnetContractAddress, limit, startAfter)
    }
    getGatewaysPaged(mixnetContractAddress: string, limit?: number, startAfter?: string): Promise<PagedGatewayResponse> {
        return this.querier.getGatewaysPaged(mixnetContractAddress, limit, startAfter)
    }
    ownsMixNode(mixnetContractAddress: string, address: string): Promise<MixOwnershipResponse> {
        return this.querier.ownsMixNode(mixnetContractAddress, address)
    }
    ownsGateway(mixnetContractAddress: string, address: string): Promise<GatewayOwnershipResponse> {
        return this.querier.ownsGateway(mixnetContractAddress, address)
    }
    getStateParams(mixnetContractAddress: string): Promise<ContractStateParams> {
        return this.querier.getStateParams(mixnetContractAddress)
    }
    getCurrentRewardingInterval(mixnetContractAddress: string): Promise<RewardingIntervalResponse> {
        return this.querier.getCurrentRewardingInterval(mixnetContractAddress)
    }

    getAllNetworkDelegationsPaged(mixnetContractAddress: string, limit?: number, startAfter?: [string, string]): Promise<PagedAllDelegationsResponse> {
        return this.querier.getAllNetworkDelegationsPaged(mixnetContractAddress, limit, startAfter)
    }
    getMixNodeDelegationsPaged(mixnetContractAddress: string, mixIdentity: string, limit?: number, startAfter?: string): Promise<PagedMixDelegationsResponse> {
        return this.querier.getMixNodeDelegationsPaged(mixnetContractAddress, mixIdentity, limit, startAfter)
    }
    getDelegatorDelegationsPaged(mixnetContractAddress: string, delegator: string, limit?: number, startAfter?: string): Promise<PagedDelegatorDelegationsResponse> {
        return this.querier.getDelegatorDelegationsPaged(mixnetContractAddress, delegator, limit, startAfter)
    }
    getDelegationDetails(mixnetContractAddress: string, mixIdentity: string, delegator: string): Promise<Delegation> {
        return this.querier.getDelegationDetails(mixnetContractAddress, mixIdentity, delegator)
    }

    getLayerDistribution(mixnetContractAddress: string): Promise<LayerDistribution> {
        return this.querier.getLayerDistribution(mixnetContractAddress)
    }
    getRewardPool(mixnetContractAddress: string): Promise<string> {
        return this.querier.getRewardPool(mixnetContractAddress)
    }
    getCirculatingSupply(mixnetContractAddress: string): Promise<string> {
        return this.querier.getCirculatingSupply(mixnetContractAddress)
    }
    getEpochRewardPercent(mixnetContractAddress: string): Promise<number> {
        return this.querier.getEpochRewardPercent(mixnetContractAddress)
    }
    getSybilResistancePercent(mixnetContractAddress: string): Promise<number> {
        return this.querier.getSybilResistancePercent(mixnetContractAddress)
    }
    getRewardingStatus(mixnetContractAddress: string, mixIdentity: string, rewardingIntervalNonce: number): Promise<RewardingStatus> {
        return this.querier.getRewardingStatus(mixnetContractAddress, mixIdentity, rewardingIntervalNonce)
    }
}
