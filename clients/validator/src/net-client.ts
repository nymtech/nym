import { SigningCosmWasmClient, SigningCosmWasmClientOptions } from "@cosmjs/cosmwasm-stargate";
import {
    GatewayOwnershipResponse,
    MixOwnershipResponse,
    PagedGatewayResponse,
    PagedResponse,
    StateParams
} from "./index";
import { DirectSecp256k1HdWallet } from "@cosmjs/proto-signing";
import { Coin, GasPrice } from "@cosmjs/launchpad";
import { BroadcastTxResponse } from "@cosmjs/stargate"
import { nymGasLimits } from "./stargate-helper"
import { ExecuteResult, InstantiateOptions, InstantiateResult, MigrateResult, UploadMeta, UploadResult } from "@cosmjs/cosmwasm";

export interface INetClient {
    clientAddress: string;

    getBalance(address: string, stakeDenom: string): Promise<Coin | null>;
    getMixNodes(contractAddress: string, limit: number, start_after?: string): Promise<PagedResponse>;
    getGateways(contractAddress: string, limit: number, start_after?: string): Promise<PagedGatewayResponse>;
    ownsMixNode(contractAddress: string, address: string): Promise<MixOwnershipResponse>;
    ownsGateway(contractAddress: string, address: string): Promise<GatewayOwnershipResponse>;
    getStateParams(contractAddress: string): Promise<StateParams>;
    executeContract(senderAddress: string, contractAddress: string, handleMsg: Record<string, unknown>, memo?: string, transferAmount?: readonly Coin[]): Promise<ExecuteResult>;
    instantiate(senderAddress: string, codeId: number, initMsg: Record<string, unknown>, label: string, options?: InstantiateOptions): Promise<InstantiateResult>;
    sendTokens(senderAddress: string, recipientAddress: string, transferAmount: readonly Coin[], memo?: string): Promise<BroadcastTxResponse>;
    upload(senderAddress: string, wasmCode: Uint8Array, meta?: UploadMeta, memo?: string): Promise<UploadResult>;
}

/**
 * Takes care of network communication between this code and the validator. 
 * Depends on `SigningCosmWasClient`, which signs all requests using keypairs 
 * derived from on bech32 mnemonics.
 * 
 * Wraps several methods from CosmWasmSigningClient so we can mock them for
 * unit testing.
 */
export default class NetClient implements INetClient {
    clientAddress: string;
    private cosmClient: SigningCosmWasmClient;
    private stakeDenom: string;

    private constructor(clientAddress: string, cosmClient: SigningCosmWasmClient, stakeDenom: string) {
        this.clientAddress = clientAddress;
        this.cosmClient = cosmClient;
        this.stakeDenom = stakeDenom;
    }

    public static async connect(wallet: DirectSecp256k1HdWallet, url: string, stakeDenom: string): Promise<INetClient> {
        const [{ address }] = await wallet.getAccounts();
        const signerOptions: SigningCosmWasmClientOptions = {
            gasPrice: GasPrice.fromString(`0.025${stakeDenom}`),
            gasLimits: nymGasLimits,
        };
        const client = await SigningCosmWasmClient.connectWithSigner(url, wallet, signerOptions);
        return new NetClient(address, client, stakeDenom);
    }

    public getMixNodes(contractAddress: string, limit: number, start_after?: string): Promise<PagedResponse> {
        if (start_after == undefined) { // TODO: check if we can take this out, I'm not sure what will happen if we send an "undefined" so I'm playing it safe here.
            return this.cosmClient.queryContractSmart(contractAddress, { get_mix_nodes: { limit } });
        } else {
            return this.cosmClient.queryContractSmart(contractAddress, { get_mix_nodes: { limit, start_after } });
        }
    }

    public getGateways(contractAddress: string, limit: number, start_after?: string): Promise<PagedGatewayResponse> {
        if (start_after == undefined) { // TODO: check if we can take this out, I'm not sure what will happen if we send an "undefined" so I'm playing it safe here.
            return this.cosmClient.queryContractSmart(contractAddress, { get_gateways: { limit } });
        } else {
            return this.cosmClient.queryContractSmart(contractAddress, { get_gateways: { limit, start_after } });
        }
    }

    public ownsMixNode(contractAddress: string, address: string): Promise<MixOwnershipResponse> {
        return this.cosmClient.queryContractSmart(contractAddress, { owns_mixnode: { address } });
    }

    public ownsGateway(contractAddress: string, address: string): Promise<GatewayOwnershipResponse> {
        return this.cosmClient.queryContractSmart(contractAddress, { owns_gateway: { address } });
    }

    public getBalance(address: string, stakeDenom: string): Promise<Coin | null> {
        return this.cosmClient.getBalance(address, stakeDenom);
    }

    public getStateParams(contractAddress: string): Promise<StateParams> {
        return this.cosmClient.queryContractSmart(contractAddress, { state_params: { } });
    }

    public executeContract(senderAddress: string, contractAddress: string, handleMsg: Record<string, unknown>, memo?: string, transferAmount?: readonly Coin[]): Promise<ExecuteResult> {
        return this.cosmClient.execute(senderAddress, contractAddress, handleMsg, memo, transferAmount);
    }

    public sendTokens(senderAddress: string, recipientAddress: string, transferAmount: readonly Coin[], memo?: string): Promise<BroadcastTxResponse> {
        return this.cosmClient.sendTokens(senderAddress, recipientAddress, transferAmount, memo);
    }

    public upload(senderAddress: string, wasmCode: Uint8Array, meta?: UploadMeta, memo?: string): Promise<UploadResult> {
        return this.cosmClient.upload(senderAddress, wasmCode, meta, memo);
    }

    public instantiate(senderAddress: string, codeId: number, initMsg: Record<string, unknown>, label: string, options?: InstantiateOptions): Promise<InstantiateResult> {
        return this.cosmClient.instantiate(senderAddress, codeId, initMsg, label, options);
    }

    public migrate(senderAddress: string, contractAddress: string, codeId: number, migrateMsg: Record<string, unknown>, memo?: string): Promise<MigrateResult> {
        return this.cosmClient.migrate(senderAddress, contractAddress, codeId, migrateMsg, memo)
    }
}
