import {
    ExecuteResult,
    InstantiateOptions,
    InstantiateResult,
    MigrateResult,
    SigningCosmWasmClient,
    SigningCosmWasmClientOptions,
    UploadResult
} from "@cosmjs/cosmwasm-stargate";
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
    RewardingIntervalResponse,
    RewardingStatus
} from "./types";
import {DirectSecp256k1HdWallet, EncodeObject} from "@cosmjs/proto-signing";
import {Coin, DeliverTxResponse, SignerData, StdFee} from "@cosmjs/stargate";
import {nymGasPrice} from "./stargate-helper"
import {IQueryClient} from "./query-client";
import {Tendermint34Client} from "@cosmjs/tendermint-rpc";
import NymdQuerier from "./nymd-querier";
import {ChangeAdminResult} from "@cosmjs/cosmwasm-stargate/build/signingcosmwasmclient";
import {TxRaw} from "cosmjs-types/cosmos/tx/v1beta1/tx";

// methods exposed by `SigningCosmWasmClient`
export interface ICosmWasmSigning {
    simulate(signerAddress: string, messages: readonly EncodeObject[], memo: string | undefined): Promise<number>;

    upload(senderAddress: string, wasmCode: Uint8Array, fee: StdFee | "auto" | number, memo?: string): Promise<UploadResult>;

    instantiate(senderAddress: string, codeId: number, msg: Record<string, unknown>, label: string, fee: StdFee | "auto" | number, options?: InstantiateOptions): Promise<InstantiateResult>;

    updateAdmin(senderAddress: string, contractAddress: string, newAdmin: string, fee: StdFee | "auto" | number, memo?: string): Promise<ChangeAdminResult>;

    clearAdmin(senderAddress: string, contractAddress: string, fee: StdFee | "auto" | number, memo?: string): Promise<ChangeAdminResult>;

    migrate(senderAddress: string, contractAddress: string, codeId: number, migrateMsg: Record<string, unknown>, fee: StdFee | "auto" | number, memo?: string): Promise<MigrateResult>;

    execute(senderAddress: string, contractAddress: string, msg: Record<string, unknown>, fee: StdFee | "auto" | number, memo?: string, funds?: readonly Coin[]): Promise<ExecuteResult>;

    sendTokens(senderAddress: string, recipientAddress: string, amount: readonly Coin[], fee: StdFee | "auto" | number, memo?: string): Promise<DeliverTxResponse>;

    delegateTokens(delegatorAddress: string, validatorAddress: string, amount: Coin, fee: StdFee | "auto" | number, memo?: string): Promise<DeliverTxResponse>;

    undelegateTokens(delegatorAddress: string, validatorAddress: string, amount: Coin, fee: StdFee | "auto" | number, memo?: string): Promise<DeliverTxResponse>;

    withdrawRewards(delegatorAddress: string, validatorAddress: string, fee: StdFee | "auto" | number, memo?: string): Promise<DeliverTxResponse>;

    signAndBroadcast(signerAddress: string, messages: readonly EncodeObject[], fee: StdFee | "auto" | number, memo?: string): Promise<DeliverTxResponse>;

    sign(signerAddress: string, messages: readonly EncodeObject[], fee: StdFee, memo: string, explicitSignerData?: SignerData): Promise<TxRaw>;
}

export interface INymSigning {
    clientAddress: string;
}

export interface ISigningClient extends IQueryClient, INymSigning {
}

/**
 * Takes care of network communication between this code and the validator.
 * Depends on `SigningCosmWasClient`, which signs all requests using keypairs
 * derived from on bech32 mnemonics.
 *
 * Wraps several methods from CosmWasmSigningClient so we can mock them for
 * unit testing.
 */
export default class SigningClient extends SigningCosmWasmClient implements ISigningClient {
    private querier: NymdQuerier;
    validatorApiUrl: string;
    clientAddress: string;

    private constructor(
        clientAddress: string,
        validatorApiUrl: string,
        tmClient: Tendermint34Client,
        wallet: DirectSecp256k1HdWallet,
        signerOptions: SigningCosmWasmClientOptions,
    ) {
        super(tmClient, wallet, signerOptions)
        this.clientAddress = clientAddress
        this.querier = new NymdQuerier(this)
        this.validatorApiUrl = validatorApiUrl
    }


    public static async connectWithNymSigner(
        wallet: DirectSecp256k1HdWallet,
        nymdUrl: string,
        validatorApiUrl: string,
        prefix: string,
    ): Promise<SigningClient> {
        const [{address}] = await wallet.getAccounts();
        const signerOptions: SigningCosmWasmClientOptions = {
            gasPrice: nymGasPrice(prefix),
        };
        const tmClient = await Tendermint34Client.connect(nymdUrl);
        return new SigningClient(address, validatorApiUrl, tmClient, wallet, signerOptions);
    }

    // query related:

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

    // signing related:


}
