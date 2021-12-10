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
    ContractSettingsParams,
    Delegation,
    GatewayOwnershipResponse,
    MixnetContractVersion,
    MixOwnershipResponse,
    PagedGatewayResponse,
    PagedMixDelegationsResponse,
    PagedMixnodeResponse
} from "./types";
import {DirectSecp256k1HdWallet, EncodeObject} from "@cosmjs/proto-signing";
import {Coin, DeliverTxResponse, SignerData, StdFee} from "@cosmjs/stargate";
import {nymGasPrice} from "./stargate-helper"
import {IQueryClient} from "./query-client";
import {Tendermint34Client} from "@cosmjs/tendermint-rpc";
import NymQuerier from "./nym-querier";
import {ChangeAdminResult} from "@cosmjs/cosmwasm-stargate/build/signingcosmwasmclient";
import {TxRaw} from "cosmjs-types/cosmos/tx/v1beta1/tx";
import {NymClient} from "./index";

export interface CosmWasmSigning {
    // methods exposed by `SigningCosmWasmClient`
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

export interface NymSigning {
    clientAddress: string;
}

export interface ISigningClient extends IQueryClient, NymSigning {
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
    private querier: NymQuerier;
    clientAddress: string;

    private constructor(
        clientAddress: string,
        tmClient: Tendermint34Client,
        wallet: DirectSecp256k1HdWallet,
        signerOptions: SigningCosmWasmClientOptions,
    ) {
        super(tmClient, wallet, signerOptions)
        this.clientAddress = clientAddress
        this.querier = new NymQuerier(this)
    }

    public static async connectWithNymSigner(
        wallet: DirectSecp256k1HdWallet,
        url: string,
        prefix: string,
    ): Promise<SigningClient> {
        const [{address}] = await wallet.getAccounts();
        const signerOptions: SigningCosmWasmClientOptions = {
            gasPrice: nymGasPrice(prefix),
        };
        const tmClient = await Tendermint34Client.connect(url);
        return new SigningClient(address, tmClient, wallet, signerOptions);
    }

    // query related:

    getContractVersion(mixnetContractAddress: string): Promise<MixnetContractVersion> {
        return this.querier.getContractVersion(mixnetContractAddress)
    }

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

    // signing related:


}
