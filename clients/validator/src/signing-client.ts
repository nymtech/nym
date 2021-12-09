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
    MixOwnershipResponse,
    PagedGatewayDelegationsResponse,
    PagedGatewayResponse,
    PagedMixDelegationsResponse,
    PagedMixnodeResponse
} from "./types";
import {DirectSecp256k1HdWallet, EncodeObject} from "@cosmjs/proto-signing";
import {Coin, DeliverTxResponse, StdFee} from "@cosmjs/stargate";
import {nymGasPrice} from "./stargate-helper"
import {IQueryClient} from "./query-client";

export interface ISigningClient extends IQueryClient {
    clientAddress: string;

    signAndBroadcast(signerAddress: string, messages: readonly EncodeObject[], fee: StdFee | "auto" | number, memo?: string): Promise<DeliverTxResponse>;

    execute(senderAddress: string, contractAddress: string, msg: Record<string, unknown>, fee: StdFee | "auto" | number, memo?: string, funds?: readonly Coin[]): Promise<ExecuteResult>;

    instantiate(senderAddress: string, codeId: number, msg: Record<string, unknown>, label: string, fee: StdFee | "auto" | number, options?: InstantiateOptions): Promise<InstantiateResult>;

    sendTokens(senderAddress: string, recipientAddress: string, amount: readonly Coin[], fee: StdFee | "auto" | number, memo?: string): Promise<DeliverTxResponse>;

    upload(senderAddress: string, wasmCode: Uint8Array, fee: StdFee | "auto" | number, memo?: string): Promise<UploadResult>;
}

/**
 * Takes care of network communication between this code and the validator.
 * Depends on `SigningCosmWasClient`, which signs all requests using keypairs
 * derived from on bech32 mnemonics.
 *
 * Wraps several methods from CosmWasmSigningClient so we can mock them for
 * unit testing.
 */
export default class SigningClient implements ISigningClient {
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

    public static async connect(wallet: DirectSecp256k1HdWallet, url: string, prefix: string): Promise<ISigningClient> {
        const [{address}] = await wallet.getAccounts();
        const signerOptions: SigningCosmWasmClientOptions = {
            gasPrice: nymGasPrice(prefix),
        };
        const client = await SigningCosmWasmClient.connectWithSigner(url, wallet, signerOptions);
        return new SigningClient(address, client, wallet, signerOptions);
    }

    async changeValidator(url: string): Promise<void> {
        this.cosmClient = await SigningCosmWasmClient.connectWithSigner(url, this.wallet, this.signerOptions);
    }

    public getMixNodes(contractAddress: string, limit: number, start_after?: string): Promise<PagedMixnodeResponse> {
        if (start_after == undefined) { // TODO: check if we can take this out, I'm not sure what will happen if we send an "undefined" so I'm playing it safe here.
            return this.cosmClient.queryContractSmart(contractAddress, {get_mix_nodes: {limit}});
        } else {
            return this.cosmClient.queryContractSmart(contractAddress, {get_mix_nodes: {limit, start_after}});
        }
    }

    public getGateways(contractAddress: string, limit: number, start_after?: string): Promise<PagedGatewayResponse> {
        if (start_after == undefined) { // TODO: check if we can take this out, I'm not sure what will happen if we send an "undefined" so I'm playing it safe here.
            return this.cosmClient.queryContractSmart(contractAddress, {get_gateways: {limit}});
        } else {
            return this.cosmClient.queryContractSmart(contractAddress, {get_gateways: {limit, start_after}});
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
        return this.cosmClient.queryContractSmart(contractAddress, {owns_mixnode: {address}});
    }

    public ownsGateway(contractAddress: string, address: string): Promise<GatewayOwnershipResponse> {
        return this.cosmClient.queryContractSmart(contractAddress, {owns_gateway: {address}});
    }

    public getBalance(address: string, denom: string): Promise<Coin | null> {
        return this.cosmClient.getBalance(address, denom);
    }

    public getContractSettingsParams(contractAddress: string): Promise<ContractSettingsParams> {
        return this.cosmClient.queryContractSmart(contractAddress, {contract_settings_params: {}});
    }

    public execute(senderAddress: string, contractAddress: string, msg: Record<string, unknown>, fee: StdFee | "auto" | number, memo?: string, funds?: readonly Coin[]): Promise<ExecuteResult> {
        return this.cosmClient.execute(senderAddress, contractAddress, msg, fee, memo);
    }

    public signAndBroadcast(signerAddress: string, messages: readonly EncodeObject[], fee: StdFee | "auto" | number, memo?: string): Promise<DeliverTxResponse> {
        return this.cosmClient.signAndBroadcast(signerAddress, messages, fee, memo)
    }

    public sendTokens(senderAddress: string, recipientAddress: string, amount: readonly Coin[], fee: StdFee | "auto" | number, memo?: string): Promise<DeliverTxResponse> {
        return this.cosmClient.sendTokens(senderAddress, recipientAddress, amount, fee, memo);
    }

    public upload(senderAddress: string, wasmCode: Uint8Array, fee: StdFee | "auto" | number, memo?: string): Promise<UploadResult> {
        return this.cosmClient.upload(senderAddress, wasmCode, fee, memo);
    }

    public instantiate(senderAddress: string, codeId: number, msg: Record<string, unknown>, label: string, fee: StdFee | "auto" | number, options?: InstantiateOptions): Promise<InstantiateResult> {
        return this.cosmClient.instantiate(senderAddress, codeId, msg, label, fee, options);
    }

    public migrate(senderAddress: string, contractAddress: string, codeId: number, migrateMsg: Record<string, unknown>, fee: StdFee | "auto" | number, memo?: string): Promise<MigrateResult> {
        return this.cosmClient.migrate(senderAddress, contractAddress, codeId, migrateMsg, fee, memo)
    }
}
