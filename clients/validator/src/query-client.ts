import {Coin} from "@cosmjs/launchpad";
import { CosmWasmClient, SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import {
    GatewayOwnershipResponse,
    MixOwnershipResponse,
    PagedGatewayResponse,
    PagedResponse,
    StateParams
} from "./index";

export interface IQueryClient {
    getBalance(address: string, stakeDenom: string): Promise<Coin | null>;
    getMixNodes(contractAddress: string, limit: number, start_after?: string): Promise<PagedResponse>;
    getGateways(contractAddress: string, limit: number, start_after?: string): Promise<PagedGatewayResponse>;
    ownsMixNode(contractAddress: string, address: string): Promise<MixOwnershipResponse>;
    ownsGateway(contractAddress: string, address: string): Promise<GatewayOwnershipResponse>;
    getStateParams(contractAddress: string): Promise<StateParams>;
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
}
