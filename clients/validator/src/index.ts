import NetClient, { INetClient } from "./net-client";
import { Gateway, GatewayBond, MixNode, MixNodeBond } from "./types";
import { Bip39, Random } from "@cosmjs/crypto";
import { DirectSecp256k1HdWallet } from "@cosmjs/proto-signing";
import MixnodesCache from "./caches/mixnodes";
import { coin, Coin, coins } from "@cosmjs/launchpad";
import { BroadcastTxResponse } from "@cosmjs/stargate"
import { ExecuteResult, InstantiateOptions, InstantiateResult, MigrateResult, UploadMeta, UploadResult } from "@cosmjs/cosmwasm";
import { CoinMap, displayAmountToNative, MappedCoin, nativeCoinToDisplay, printableBalance, printableCoin } from "./currency";
import GatewaysCache from "./caches/gateways";
import QueryClient, { IQueryClient } from "./query-client";

export { coins, coin };
export { Coin };
export { displayAmountToNative, nativeCoinToDisplay, printableCoin, printableBalance, MappedCoin, CoinMap }

export default class ValidatorClient {
    private readonly stakeDenom: string;
    private readonly gatewayBondingStake: number = 1000_000000
    url: string;
    private readonly client: INetClient | IQueryClient
    private mixNodesCache: MixnodesCache;
    private gatewayCache: GatewaysCache
    private readonly contractAddress: string;

    private constructor(url: string, client: INetClient | IQueryClient, contractAddress: string, stakeDenom: string) {
        this.url = url;
        this.client = client;
        this.mixNodesCache = new MixnodesCache(client, 100);
        this.gatewayCache = new GatewaysCache(client, 100);
        this.contractAddress = contractAddress;
        this.stakeDenom = stakeDenom;
    }

    static async connect(contractAddress: string, mnemonic: string, url: string, stakeDenom: string): Promise<ValidatorClient> {
        const wallet = await ValidatorClient.buildWallet(mnemonic);
        const netClient = await NetClient.connect(wallet, url, stakeDenom);
        return new ValidatorClient(url, netClient, contractAddress, stakeDenom);
    }

    static async connectForQuery(contractAddress: string, url: string, stakeDenom: string): Promise<ValidatorClient> {
        const queryClient = await QueryClient.connect(url)
        return new ValidatorClient(url, queryClient, contractAddress, stakeDenom)
    }

    public get address(): string {
        if (this.client instanceof NetClient) {
            return this.client.clientAddress
        } else {
            return ""
        }
    }

    /**
     * TODO: re-enable this once we move back to client-side wallets running on people's machines
     * instead of the web wallet.
     *
     * Loads a named mnemonic from the system's keystore.
     *
     * @param keyPath the name of the key in the keystore
     * @returns the mnemonic as a string
     */
    // static loadMnemonic(keyPath: string): string {
    //     try {
    //         const mnemonic = fs.readFileSync(keyPath, "utf8");
    //         return mnemonic.trim();
    //     } catch (err) {
    //         console.log(err);
    //         return "fight with type system later";
    //     }
    // }

    /**
     * Generates a random mnemonic, useful for creating new accounts.
     * @returns a fresh mnemonic.
     */
    static randomMnemonic(): string {
        return Bip39.encode(Random.getBytes(32)).toString();
    }

    /**
     * @param mnemonic A mnemonic from which to generate a public/private keypair.
     * @returns the address for this client wallet
     */
    static async mnemonicToAddress(mnemonic: string): Promise<string> {
        const wallet = await ValidatorClient.buildWallet(mnemonic);
        const [{ address }] = await wallet.getAccounts()
        return address
    }

    static async buildWallet(mnemonic: string): Promise<DirectSecp256k1HdWallet> {
        return DirectSecp256k1HdWallet.fromMnemonic(mnemonic, undefined, "hal");
    }

    getBalance(address: string): Promise<Coin | null> {
        return this.client.getBalance(address, this.stakeDenom);
    }

    async getStateParams(): Promise<StateParams> {
        return this.client.getStateParams(this.contractAddress)
    }


    /**
     * Get or refresh the list of mixnodes in the network.
     *
     * @returns an array containing all known `MixNodeBond`s.
     *
     * TODO: We will want to put this puppy on a timer, but for the moment we can
     * just get things strung together and refresh it manually.
     */
    refreshMixNodes(): Promise<MixNodeBond[]> {
        return this.mixNodesCache.refreshMixNodes(this.contractAddress);
    }

    /**
     * Get mixnodes from the local client cache.
     *
     * @returns an array containing all `MixNodeBond`s in the client's local cache.
     */
    getMixNodes(): MixNodeBond[] {
        return this.mixNodesCache.mixNodes
    }

    /**
    *  Announce a mixnode, paying a fee.
    */
    async bond(mixNode: MixNode): Promise<ExecuteResult> {
        if (this.client instanceof NetClient) {
            const bond = [{ amount: "1000000000", denom: this.stakeDenom }];
            const result = await this.client.executeContract(this.client.clientAddress, this.contractAddress, { register_mixnode: { mix_node: mixNode } }, "adding mixnode", bond);
            console.log(`account ${this.client.clientAddress} added mixnode with ${mixNode.host}`);
            return result;
        } else {
            throw new Error("Tried to bond with a query client")
        }

    }

    /**
     * Unbond a mixnode, removing it from the network and reclaiming staked coins
     */
    async unbond(): Promise<ExecuteResult> {
        if (this.client instanceof NetClient) {
            const result = await this.client.executeContract(this.client.clientAddress, this.contractAddress, { un_register_mixnode: {} })
            console.log(`account ${this.client.clientAddress} unbonded mixnode`);
            return result;
        } else {
            throw new Error("Tried to unbond with a query client")
        }
    }

    /**
     * Checks whether there is already a bonded mixnode associated with this client's address
     */
    async ownsMixNode(): Promise<boolean> {
        if (this.client instanceof NetClient) {
            const result = await this.client.ownsMixNode(this.contractAddress, this.client.clientAddress)
            return result.has_node
        } else {
            throw new Error("tried to check mixnode ownership for an address-less client")
        }
    }

    /**
     * Checks whether there is already a bonded gateway associated with this client's address
     */
    async ownsGateway(): Promise<boolean> {
        if (this.client instanceof NetClient) {
            const result = await this.client.ownsGateway(this.contractAddress, this.client.clientAddress)
            return result.has_gateway
        } else {
            throw new Error("tried to check gateway ownership for an address-less client")
        }
    }

    /**
     * Get or refresh the list of gateways in the network.
     *
     * @returns an array containing all known `GatewayBond`s.
     *
     * TODO: Similarly to mixnode bonds, this should probably be put on a timer somewhere.
     */
    refreshGateways(): Promise<GatewayBond[]> {
        return this.gatewayCache.refreshGateways(this.contractAddress);
    }

    /**
     * Get gateways from the local client cache.
     *
     * @returns an array containing all `GatewayBond`s in the client's local cache.
     */
    getGateways(): GatewayBond[] {
        return this.gatewayCache.gateways
    }

    /**
     * Generate a minimum gateway bond required to create a fresh gateway.
     *
     * @returns a `Coin` instance containing minimum amount of coins to stake a gateway.
     */
    minimumGatewayBond = (): Coin => {
        return coin(this.gatewayBondingStake, this.stakeDenom)
    }

    /**
     *  Announce a gateway, paying a fee.
     */
    async bondGateway(gateway: Gateway): Promise<ExecuteResult> {
        if (this.client instanceof NetClient) {
            const bond = this.minimumGatewayBond()
            const result = await this.client.executeContract(this.client.clientAddress, this.contractAddress, { bond_gateway: { gateway: gateway } }, "adding gateway", [bond]);
            console.log(`account ${this.client.clientAddress} added gateway with ${gateway.mix_host}`);
            return result;
        } else {
            throw new Error("Tried to bond gateway with a query client")
        }
    }

    /**
     * Unbond a gateway, removing it from the network and reclaiming staked coins
     */
    async unbondGateway(): Promise<ExecuteResult> {
        if (this.client instanceof NetClient) {
            const result = await this.client.executeContract(this.client.clientAddress, this.contractAddress, { unbond_gateway: {} })
            console.log(`account ${this.client.clientAddress} unbonded gateway`);
            return result;
        } else {
            throw new Error("Tried to unbond gateway with a query client")
        }
    }


    // TODO: if we just keep a reference to the SigningCosmWasmClient somewhere we can probably go direct
    // to it in the case of these methods below.

    /**
     * Send funds from one address to another.
     */
    async send(senderAddress: string, recipientAddress: string, coins: readonly Coin[], memo?: string): Promise<BroadcastTxResponse> {
        if (this.client instanceof NetClient) {
            return this.client.sendTokens(senderAddress, recipientAddress, coins, memo);
        } else {
            throw new Error("Tried to use send with a query client");
        }
    }

    async upload(senderAddress: string, wasmCode: Uint8Array, meta?: UploadMeta, memo?: string): Promise<UploadResult> {
        if (this.client instanceof NetClient) {
            return this.client.upload(senderAddress, wasmCode, meta, memo);
        } else {
            throw new Error("Tried to upload with a query client");
        }
    }

    public instantiate(senderAddress: string, codeId: number, initMsg: Record<string, unknown>, label: string, options?: InstantiateOptions): Promise<InstantiateResult> {
        if (this.client instanceof NetClient) {
            return this.client.instantiate(senderAddress, codeId, initMsg, label, options);
        } else {
            throw new Error("Tried to instantiate with a query client");
        }
    }

    public migrate(senderAddress: string, contractAddress: string, codeId: number, migrateMsg: Record<string, unknown>, memo?: string): Promise<MigrateResult> {
        if (this.client instanceof NetClient) {
            return this.client.migrate(senderAddress, contractAddress, codeId, migrateMsg, memo)
        } else {
            throw new Error("Tried to migrate with a query client");
        }
    }
}


/// One page of a possible multi-page set of mixnodes. The paging interface is quite
/// inconvenient, as we don't have the two pieces of information we need to know
/// in order to do paging nicely (namely `currentPage` and `totalPages` parameters).
///
/// Instead, we have only `start_next_page_after`, i.e. the key of the last record
/// on this page. In order to get the *next* page, CosmWasm looks at that value,
/// finds the next record, and builds the next page starting there. This happens
/// **in series** rather than **in parallel** (!).
///
/// So we have some consistency problems:
///
/// * we can't make requests at a given block height, so the result set
///    which we assemble over time may change while requests are being made.
/// * at some point we will make a request for a `start_next_page_after` key
///   which has just been deleted from the database.
///
/// TODO: more robust error handling on the "deleted key" case.
export type PagedResponse = {
    nodes: MixNodeBond[],
    per_page: number, // TODO: camelCase
    start_next_after: string, // TODO: camelCase
}

// a temporary way of achieving the same paging behaviour for the gateways
// the same points made for `PagedResponse` stand here.
export type PagedGatewayResponse = {
    nodes: GatewayBond[],
    per_page: number, // TODO: camelCase
    start_next_after: string, // TODO: camelCase
}

export type MixOwnershipResponse = {
    address: string,
    has_node: boolean,
}

export type GatewayOwnershipResponse = {
    address: string,
    has_gateway: boolean,
}

export type StateParams = {
    minimum_mixnode_bond: number,
    minimum_gateway_bond: number,
    mixnode_bond_reward_rate: number,
    gateway_bond_reward_rate: number,
    mixnode_active_set_size: number,
}
