import { SigningCosmWasmClient, SigningCosmWasmClientOptions } from "@cosmjs/cosmwasm-stargate";
import {
    Delegation,
    GatewayOwnershipResponse,
    MixOwnershipResponse, PagedGatewayDelegationsResponse,
    PagedGatewayResponse, PagedMixDelegationsResponse,
    PagedMixnodeResponse,
    StateParams
} from "./types";
import { DirectSecp256k1HdWallet, EncodeObject } from "@cosmjs/proto-signing";
import { Coin, StdFee } from "@cosmjs/stargate";
import { BroadcastTxResponse } from "@cosmjs/stargate"
import { nymGasLimits, nymGasPrice } from "./stargate-helper"
import {
    ExecuteResult,
    InstantiateOptions,
    InstantiateResult,
    MigrateResult,
    UploadMeta,
    UploadResult
} from "@cosmjs/cosmwasm-stargate";

export interface INetClient {
    clientAddress: string;

    getBalance(address: string, denom: string): Promise<Coin | null>;

    getMixNodes(contractAddress: string, limit: number, start_after?: string): Promise<PagedMixnodeResponse>;

    getGateways(contractAddress: string, limit: number, start_after?: string): Promise<PagedGatewayResponse>;

    getMixDelegations(contractAddress: string, mixIdentity: string, limit: number, start_after?: string): Promise<PagedMixDelegationsResponse>

    getMixDelegation(contractAddress: string, mixIdentity: string, delegatorAddress: string): Promise<Delegation>

    getGatewayDelegations(contractAddress: string, gatewayIdentity: string, limit: number, start_after?: string): Promise<PagedGatewayDelegationsResponse>

    getGatewayDelegation(contractAddress: string, gatewayIdentity: string, delegatorAddress: string): Promise<Delegation>

    ownsMixNode(contractAddress: string, address: string): Promise<MixOwnershipResponse>;

    ownsGateway(contractAddress: string, address: string): Promise<GatewayOwnershipResponse>;

    getStateParams(contractAddress: string): Promise<StateParams>;

    signAndBroadcast(signerAddress: string, messages: readonly EncodeObject[], fee: StdFee, memo?: string): Promise<BroadcastTxResponse>;

    executeContract(senderAddress: string, contractAddress: string, handleMsg: Record<string, unknown>, memo?: string, transferAmount?: readonly Coin[]): Promise<ExecuteResult>;

    instantiate(senderAddress: string, codeId: number, initMsg: Record<string, unknown>, label: string, options?: InstantiateOptions): Promise<InstantiateResult>;

    sendTokens(senderAddress: string, recipientAddress: string, transferAmount: readonly Coin[], memo?: string): Promise<BroadcastTxResponse>;

    upload(senderAddress: string, wasmCode: Uint8Array, meta?: UploadMeta, memo?: string): Promise<UploadResult>;

    changeValidator(newUrl: string): Promise<void>
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

    // helpers for changing validators without having to remake the wallet
    private readonly wallet: DirectSecp256k1HdWallet;
    private readonly signerOptions: SigningCosmWasmClientOptions;

    private constructor(clientAddress: string, cosmClient: SigningCosmWasmClient, wallet: DirectSecp256k1HdWallet, signerOptions: SigningCosmWasmClientOptions) {
        this.clientAddress = clientAddress;
        this.cosmClient = cosmClient;
        this.wallet = wallet;
        this.signerOptions = signerOptions;
    }

    public static async connect(wallet: DirectSecp256k1HdWallet, url: string, prefix: string): Promise<INetClient> {
        const [{ address }] = await wallet.getAccounts();
        const signerOptions: SigningCosmWasmClientOptions = {
            gasPrice: nymGasPrice(prefix),
            gasLimits: nymGasLimits,
        };
        const client = await SigningCosmWasmClient.connectWithSigner(url, wallet, signerOptions);
        return new NetClient(address, client, wallet, signerOptions);
    }

    async changeValidator(url: string): Promise<void> {
        this.cosmClient = await SigningCosmWasmClient.connectWithSigner(url, this.wallet, this.signerOptions);
    }

    public getMixNodes(contractAddress: string, limit: number, start_after?: string): Promise<PagedMixnodeResponse> {
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

    public getMixDelegations(contractAddress: string, mixIdentity: string, limit: number, start_after?: string): Promise<PagedMixDelegationsResponse> {
        if (start_after == undefined) { // TODO: check if we can take this out, I'm not sure what will happen if we send an "undefined" so I'm playing it safe here.
            return this.cosmClient.queryContractSmart(contractAddress, {
                get_mix_delegations: {
                    mix_identity: mixIdentity,
                    limit
                }
            });
        } else {
            return this.cosmClient.queryContractSmart(contractAddress, {
                get_mix_delegations: {
                    mix_identity: mixIdentity,
                    limit,
                    start_after
                }
            });
        }
    }

    public getMixDelegation(contractAddress: string, mixIdentity: string, delegatorAddress: string): Promise<Delegation> {
        return this.cosmClient.queryContractSmart(contractAddress, {
            get_mix_delegation: {
                mix_identity: mixIdentity,
                address: delegatorAddress
            }
        });
    }

    public getGatewayDelegations(contractAddress: string, gatewayIdentity: string, limit: number, start_after?: string): Promise<PagedGatewayDelegationsResponse> {
        if (start_after == undefined) { // TODO: check if we can take this out, I'm not sure what will happen if we send an "undefined" so I'm playing it safe here.
            return this.cosmClient.queryContractSmart(contractAddress, {
                get_gateway_delegations: {
                    gateway_identity: gatewayIdentity,
                    limit
                }
            });
        } else {
            return this.cosmClient.queryContractSmart(contractAddress, {
                get_gateway_delegations: {
                    gateway_identity: gatewayIdentity,
                    limit,
                    start_after
                }
            });
        }
    }

    public getGatewayDelegation(contractAddress: string, gatewayIdentity: string, delegatorAddress: string): Promise<Delegation> {
        return this.cosmClient.queryContractSmart(contractAddress, {
            get_gateway_delegation: {
                gateway_identity: gatewayIdentity,
                address: delegatorAddress
            }
        });
    }

    public ownsMixNode(contractAddress: string, address: string): Promise<MixOwnershipResponse> {
        return this.cosmClient.queryContractSmart(contractAddress, { owns_mixnode: { address } });
    }

    public ownsGateway(contractAddress: string, address: string): Promise<GatewayOwnershipResponse> {
        return this.cosmClient.queryContractSmart(contractAddress, { owns_gateway: { address } });
    }

    public getBalance(address: string, denom: string): Promise<Coin | null> {
        return this.cosmClient.getBalance(address, denom);
    }

    public getStateParams(contractAddress: string): Promise<StateParams> {
        return this.cosmClient.queryContractSmart(contractAddress, { contract_settings_params: {} });
    }

    public executeContract(senderAddress: string, contractAddress: string, handleMsg: Record<string, unknown>, memo?: string, transferAmount?: readonly Coin[]): Promise<ExecuteResult> {
        return this.cosmClient.execute(senderAddress, contractAddress, handleMsg, memo, transferAmount);
    }

    public signAndBroadcast(signerAddress: string, messages: readonly EncodeObject[], fee: StdFee, memo?: string): Promise<BroadcastTxResponse> {
        return this.cosmClient.signAndBroadcast(signerAddress, messages, fee, memo)
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
