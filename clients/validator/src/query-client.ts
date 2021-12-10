import {CosmWasmClient} from "@cosmjs/cosmwasm-stargate";
import {
    ContractSettingsParams,
    Delegation,
    GatewayOwnershipResponse, MixnetContractVersion,
    MixOwnershipResponse,
    PagedGatewayResponse,
    PagedMixDelegationsResponse,
    PagedMixnodeResponse
} from "./types";
import {Tendermint34Client} from "@cosmjs/tendermint-rpc";
import NymQuerier from "./nym-querier";
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
import {NymClient} from "./index";

export interface CosmWasmQuery {
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

export interface NymQuery {
    // nym-specific implemented inside NymQuerier
    getContractVersion(mixnetContractAddress: string): Promise<MixnetContractVersion>;

    // getMixNodes(contractAddress: string, limit: number, start_after?: string): Promise<PagedMixnodeResponse>;
    // getGateways(contractAddress: string, limit: number, start_after?: string): Promise<PagedGatewayResponse>;
    // getMixDelegations(contractAddress: string, mixIdentity: string, limit: number, start_after?: string): Promise<PagedMixDelegationsResponse>
    // getMixDelegation(contractAddress: string, mixIdentity: string, delegatorAddress: string): Promise<Delegation>
    // ownsMixNode(contractAddress: string, address: string): Promise<MixOwnershipResponse>;
    // ownsGateway(contractAddress: string, address: string): Promise<GatewayOwnershipResponse>;
    // getContractSettingsParams(contractAddress: string): Promise<ContractSettingsParams>;

    /*
    GetContractVersion {},
    GetMixNodes {
        limit: Option<u32>,
        start_after: Option<IdentityKey>,
    },
    GetGateways {
        start_after: Option<IdentityKey>,
        limit: Option<u32>,
    },
    OwnsMixnode {
        address: String,
    },
    OwnsGateway {
        address: String,
    },
    StateParams {},
    CurrentRewardingInterval {},
    // gets all [paged] delegations in the entire network
    // TODO: do we even want that?
    GetAllNetworkDelegations {
        start_after: Option<(IdentityKey, String)>,
        limit: Option<u32>,
    },
    // gets all [paged] delegations associated with particular mixnode
    GetMixnodeDelegations {
        mix_identity: IdentityKey,
        // since `start_after` is user-provided input, we can't use `Addr` as we
        // can't guarantee it's validated.
        start_after: Option<String>,
        limit: Option<u32>,
    },
    // gets all [paged] delegations associated with particular delegator
    GetDelegatorDelegations {
        // since `delegator` is user-provided input, we can't use `Addr` as we
        // can't guarantee it's validated.
        delegator: String,
        start_after: Option<IdentityKey>,
        limit: Option<u32>,
    },
    // gets delegation associated with particular mixnode, delegator pair
    GetDelegationDetails {
        mix_identity: IdentityKey,
        delegator: String,
    },
    LayerDistribution {},
    GetRewardPool {},
    GetCirculatingSupply {},
    GetEpochRewardPercent {},
    GetSybilResistancePercent {},
    GetRewardingStatus {
        mix_identity: IdentityKey,
        rewarding_interval_nonce: u32,
    },
     */

}

export interface IQueryClient extends CosmWasmQuery, NymQuery{}

/**
 * Takes care of network communication between this code and the validator.
 * Depends on `SigningCosmWasClient`, which signs all requests using keypairs
 * derived from on bech32 mnemonics.
 *
 * Wraps several methods from CosmWasmSigningClient so we can mock them for
 * unit testing.
 */
export default class QueryClient extends CosmWasmClient implements IQueryClient {
    private querier: NymQuerier;

    private constructor(tmClient: Tendermint34Client) {
        super(tmClient)
        this.querier = new NymQuerier(this)
    }


    public static async connectWithNym(url: string): Promise<QueryClient> {
        const tmClient = await Tendermint34Client.connect(url);
        return new QueryClient(tmClient)
    }


    getContractVersion(mixnetContractAddress: string): Promise<MixnetContractVersion> {
        return this.querier.getContractVersion(mixnetContractAddress)
    }

    // // change it to anonymous functions
    // public getMixNodes(contractAddress: string, limit: number, start_after?: string): Promise<PagedMixnodeResponse> {
    //     return this.querier.getMixNodes(contractAddress, limit, start_after);
    // }
    //
    // public getGateways(contractAddress: string, limit: number, start_after?: string): Promise<PagedGatewayResponse> {
    //     return this.querier.getGateways(contractAddress, limit, start_after)
    // }
    //
    // public getMixDelegations(contractAddress: string, mixIdentity: string, limit: number, start_after?: string): Promise<PagedMixDelegationsResponse> {
    //     return this.querier.getMixDelegations(contractAddress, mixIdentity, limit, start_after);
    // }
    //
    // public getMixDelegation(contractAddress: string, mixIdentity: string, delegatorAddress: string): Promise<Delegation> {
    //     return this.querier.getMixDelegation(contractAddress, mixIdentity, delegatorAddress);
    // }
    //
    // public ownsMixNode(contractAddress: string, address: string): Promise<MixOwnershipResponse> {
    //     return this.querier.ownsMixNode(contractAddress, address);
    // }
    //
    // public ownsGateway(contractAddress: string, address: string): Promise<GatewayOwnershipResponse> {
    //     return this.querier.ownsGateway(contractAddress, address);
    // }
    //
    // public getContractSettingsParams(contractAddress: string): Promise<ContractSettingsParams> {
    //     return this.querier.getContractSettingsParams(contractAddress);
    // }
}
