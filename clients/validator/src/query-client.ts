import { Coin } from "@cosmjs/stargate";
import { CosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import {
    Delegation,
    GatewayOwnershipResponse,
    MixOwnershipResponse, PagedGatewayDelegationsResponse,
    PagedGatewayResponse, PagedMixDelegationsResponse,
    PagedMixnodeResponse,
    ContractSettingsParams
} from "./types";

export interface IQueryClient {
    getBalance(address: string, stakeDenom: string): Promise<Coin | null>;

    getMixNodes(contractAddress: string, limit: number, start_after?: string): Promise<PagedMixnodeResponse>;

    getGateways(contractAddress: string, limit: number, start_after?: string): Promise<PagedGatewayResponse>;

    getMixDelegations(contractAddress: string, mixIdentity: string, limit: number, start_after?: string): Promise<PagedMixDelegationsResponse>

    getMixDelegation(contractAddress: string, mixIdentity: string, delegatorAddress: string): Promise<Delegation>

    getGatewayDelegations(contractAddress: string, gatewayIdentity: string, limit: number, start_after?: string): Promise<PagedGatewayDelegationsResponse>

    getGatewayDelegation(contractAddress: string, gatewayIdentity: string, delegatorAddress: string): Promise<Delegation>

    ownsMixNode(contractAddress: string, address: string): Promise<MixOwnershipResponse>;

    ownsGateway(contractAddress: string, address: string): Promise<GatewayOwnershipResponse>;

    getStateParams(contractAddress: string): Promise<ContractSettingsParams>;

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
export default class QueryClient implements IQueryClient {
    private cosmClient: CosmWasmClient;

    private constructor(cosmClient: CosmWasmClient) {
        this.cosmClient = cosmClient;
    }

    public static async connect(url: string): Promise<IQueryClient> {
        const client = await CosmWasmClient.connect(url)
        return new QueryClient(client)
    }

    async changeValidator(url: string): Promise<void> {
        this.cosmClient = await CosmWasmClient.connect(url)
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

    public getBalance(address: string, stakeDenom: string): Promise<Coin | null> {
        return this.cosmClient.getBalance(address, stakeDenom);
    }

    public getStateParams(contractAddress: string): Promise<ContractSettingsParams> {
        return this.cosmClient.queryContractSmart(contractAddress, { contract_settings_params: {} });
    }
}
