import NetClient, { INetClient } from "./net-client";
import { Gateway, GatewayBond, MixNode, MixNodeBond, SendRequest } from "./types";
import { Bip39, Random } from "@cosmjs/crypto";
import { DirectSecp256k1HdWallet, EncodeObject } from "@cosmjs/proto-signing";
import MixnodesCache from "./caches/mixnodes";
import { buildFeeTable, coin, Coin, coins, StdFee } from "@cosmjs/launchpad";
import {
    ExecuteResult,
    InstantiateOptions,
    InstantiateResult,
    MigrateResult,
    UploadMeta,
    UploadResult
} from "@cosmjs/cosmwasm";
import {
    CoinMap,
    displayAmountToNative,
    MappedCoin,
    nativeCoinToDisplay,
    printableBalance,
    printableCoin,
    nativeToPrintable
} from "./currency";
import GatewaysCache from "./caches/gateways";
import QueryClient, { IQueryClient } from "./query-client";
import { nymGasLimits, nymGasPrice } from "./stargate-helper";
import { BroadcastTxSuccess, isBroadcastTxFailure } from "@cosmjs/stargate";
import { makeBankMsgSend } from "./utils";

export { coins, coin };
export { Coin };
export { displayAmountToNative, nativeCoinToDisplay, printableCoin, printableBalance, nativeToPrintable, MappedCoin, CoinMap }
export { nymGasLimits, nymGasPrice }

export default class ValidatorClient {
    private readonly stakeDenom: string;
    // TODO: do those even still make sense since they can vary?
    private readonly defaultGatewayBondingStake: number = 100_000000
    private readonly defaultMixnodeBondingStake: number = 100_000000

    urls: string[];
    private readonly client: INetClient | IQueryClient
    private mixNodesCache: MixnodesCache;
    private gatewayCache: GatewaysCache
    private readonly contractAddress: string;
    // for some reason typescript thinks it's better to not be explicit about a trivial type...
    // eslint-disable-next-line @typescript-eslint/no-inferrable-types
    private failedRequests: number = 0;

    private constructor(urls: string[], client: INetClient | IQueryClient, contractAddress: string, stakeDenom: string) {
        this.urls = urls;
        this.client = client;
        this.mixNodesCache = new MixnodesCache(client, 100);
        this.gatewayCache = new GatewaysCache(client, 100);
        this.contractAddress = contractAddress;
        this.stakeDenom = stakeDenom;
    }

    // allows also entering 'string' by itself for backwards compatibility
    static async connect(contractAddress: string, mnemonic: string, urls: string | string[], stakeDenom: string): Promise<ValidatorClient> {
        const validatorUrls = this.dealWithValidatorUrls(urls)
        const wallet = await ValidatorClient.buildWallet(mnemonic);

        // if we have more than a single validator, try to perform initial connection until we succeed or run out of options
        if (validatorUrls.length > 1) {
            for (let i = 0; i < validatorUrls.length; i++) {
                console.log("Attempting initial connection to", validatorUrls[0])
                const netClient = await NetClient.connect(wallet, validatorUrls[0], stakeDenom).catch((_) => ValidatorClient.moveArrayHeadToBack(validatorUrls))
                if (netClient !== undefined) {
                    return new ValidatorClient(validatorUrls, netClient, contractAddress, stakeDenom);
                }
                console.log("Initial connection to", validatorUrls[0], "failed")
            }
        } else {
            const netClient = await NetClient.connect(wallet, validatorUrls[0], stakeDenom)
            return new ValidatorClient(validatorUrls, netClient, contractAddress, stakeDenom);
        }

        throw new Error("None of the provided validators seem to be alive")
    }

    // allows also entering 'string' by itself for backwards compatibility
    static async connectForQuery(contractAddress: string, urls: string | string[], stakeDenom: string): Promise<ValidatorClient> {
        const validatorUrls = this.dealWithValidatorUrls(urls)

        // if we have more than a single validator, try to perform initial connection until we succeed or run out of options
        if (validatorUrls.length > 1) {
            for (let i = 0; i < validatorUrls.length; i++) {
                console.log("Attempting initial connection to", validatorUrls[0])
                const queryClient = await QueryClient.connect(validatorUrls[0]).catch((_) => ValidatorClient.moveArrayHeadToBack(validatorUrls))
                if (queryClient !== undefined) {
                    return new ValidatorClient(validatorUrls, queryClient, contractAddress, stakeDenom)
                }
                console.log("Initial connection to", validatorUrls[0], "failed")
            }
        } else {
            const queryClient = await QueryClient.connect(validatorUrls[0])
            return new ValidatorClient(validatorUrls, queryClient, contractAddress, stakeDenom)
        }

        throw new Error("None of the provided validators seem to be alive")
    }

    private static dealWithValidatorUrls(urls: string | string[]): string[] {
        let validatorsUrls: string[] = []
        if (typeof urls === "string") {
            validatorsUrls = [urls]
        } else {
            // if the array is empty, just blow up
            if (urls.length === 0) {
                throw new Error("no validator urls provided")
            }

            // no point in shuffling array of size 1
            if (urls.length > 1) {
                urls = this.shuffleArray(urls)
            }
            validatorsUrls = urls
        }

        return validatorsUrls
    }

    // an error adapter function that upon an error attempts to switch currently used validator to the next one available
    // note that it ALWAYS throws an error
    async handleRequestFailure(error: Error): Promise<never> {
        // don't bother doing any fancy validator switches if we only have 1 validator to choose from
        if (this.urls.length > 1) {
            this.failedRequests += 1;
            // if we exhausted all of available validators, permute the set, maybe the old ones
            // are working again next time we try
            if (this.failedRequests === this.urls.length) {
                this.urls = ValidatorClient.shuffleArray(this.urls)
            } else {
                // otherwise change the front validator to a 'fresh' one
                // during construction we assured we don't have an empty array
                ValidatorClient.moveArrayHeadToBack(this.urls)
            }
            // and change validator to the front one and rethrow the error
            return await this.changeValidator(this.urls[0]).then(() => {throw error})
        } else {
            // rethrow the error
            throw error
        }
    }

    private async changeValidator(newUrl: string): Promise<void> {
        console.log("Changing validator to", newUrl)
        return await this.client.changeValidator(newUrl)
    }

    // adapted from https://stackoverflow.com/questions/6274339/how-can-i-shuffle-an-array/6274381#6274381
    static shuffleArray<T>(arr: T[]): T[] {
        for (let i = arr.length - 1; i > 0; i--) {
            const j = Math.floor(Math.random() * (i + 1));
            [arr[i], arr[j]] = [arr[j], arr[i]];
        }
        return arr;
    }

    // It is responsibility of the caller to ensure the input array is non-empty
    private static moveArrayHeadToBack<T>(arr: T[]) {
        const head = <T> arr.shift()
        arr.push(head)
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
        const [{address}] = await wallet.getAccounts()
        return address
    }

    static async buildWallet(mnemonic: string): Promise<DirectSecp256k1HdWallet> {
        return DirectSecp256k1HdWallet.fromMnemonic(mnemonic, undefined, "hal");
    }

    getBalance(address: string): Promise<Coin | null> {
        return this.client.getBalance(address, this.stakeDenom).catch((err) => this.handleRequestFailure(err));
    }

    async getStateParams(): Promise<StateParams> {
        return this.client.getStateParams(this.contractAddress).catch((err) => this.handleRequestFailure(err))
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
        return this.mixNodesCache.refreshMixNodes(this.contractAddress).catch((err) => this.handleRequestFailure(err));
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
     * Generate a minimum gateway bond required to create a fresh mixnode.
     *
     * @returns a `Coin` instance containing minimum amount of coins to stake a gateway.
     */
    minimumMixnodeBond = (): Coin => {
        return coin(this.defaultMixnodeBondingStake, this.stakeDenom)
    }

    /**
     *  Announce a mixnode, paying a fee.
     */
    async bondMixnode(mixNode: MixNode): Promise<ExecuteResult> {
        if (this.client instanceof NetClient) {
            const bond = [this.minimumMixnodeBond()];
            const result = await this.client.executeContract(this.client.clientAddress, this.contractAddress, { bond_mixnode: { mix_node: mixNode } }, "adding mixnode", bond).catch((err) => this.handleRequestFailure(err));
            console.log(`account ${this.client.clientAddress} added mixnode with ${mixNode.host}`);
            return result;
        } else {
            throw new Error("Tried to bond with a query client")
        }

    }

    /**
     * Unbond a mixnode, removing it from the network and reclaiming staked coins
     */
    async unbondMixnode(): Promise<ExecuteResult> {
        if (this.client instanceof NetClient) {
            const result = await this.client.executeContract(this.client.clientAddress, this.contractAddress, { unbond_mixnode: {} }).catch((err) => this.handleRequestFailure(err))
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
            const result = await this.client.ownsMixNode(this.contractAddress, this.client.clientAddress).catch((err) => this.handleRequestFailure(err))
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
            const result = await this.client.ownsGateway(this.contractAddress, this.client.clientAddress).catch((err) => this.handleRequestFailure(err))
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
        return this.gatewayCache.refreshGateways(this.contractAddress).catch((err) => this.handleRequestFailure(err));
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
        return coin(this.defaultGatewayBondingStake, this.stakeDenom)
    }

    /**
     *  Announce a gateway, paying a fee.
     */
    async bondGateway(gateway: Gateway): Promise<ExecuteResult> {
        if (this.client instanceof NetClient) {
            const bond = this.minimumGatewayBond()
            const result = await this.client.executeContract(this.client.clientAddress, this.contractAddress, {bond_gateway: {gateway: gateway}}, "adding gateway", [bond]).catch((err) => this.handleRequestFailure(err));
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
            const result = await this.client.executeContract(this.client.clientAddress, this.contractAddress, {unbond_gateway: {}}).catch((err) => this.handleRequestFailure(err))
            console.log(`account ${this.client.clientAddress} unbonded gateway`);
            return result;
        } else {
            throw new Error("Tried to unbond gateway with a query client")
        }
    }

    async updateStateParams(newParams: StateParams): Promise<ExecuteResult> {
        if (this.client instanceof NetClient) {
            return await this.client.executeContract(this.client.clientAddress, this.contractAddress, {update_state_params: newParams}, "updating contract state").catch((err) => this.handleRequestFailure(err));
        } else {
            throw new Error("Tried to update state params with a query client")
        }
    }

    // TODO: if we just keep a reference to the SigningCosmWasmClient somewhere we can probably go direct
    // to it in the case of these methods below.

    /**
     * Send funds from one address to another.
     */
    async send(senderAddress: string, recipientAddress: string, coins: readonly Coin[], memo?: string): Promise<BroadcastTxSuccess> {
        if (this.client instanceof NetClient) {
            const result = await this.client.sendTokens(senderAddress, recipientAddress, coins, memo).catch((err) => this.handleRequestFailure(err));
            if (isBroadcastTxFailure(result)) {
                throw new Error(`Error when broadcasting tx ${result.transactionHash} at height ${result.height}. Code: ${result.code}; Raw log: ${result.rawLog}`)
            }
            return result
        } else {
            throw new Error("Tried to use send with a query client");
        }
    }

    /**
     * Send funds multiple times from one address to another in a single block.
    */
    async sendMultiple(senderAddress: string, data: SendRequest[], memo?: string): Promise<BroadcastTxSuccess> {
        if (this.client instanceof NetClient) {
            if (data.length === 1) {
                return this.send(data[0].senderAddress, data[0].recipientAddress, data[0].transferAmount, memo)
            }

            const encoded = data.map(req => makeBankMsgSend(req.senderAddress, req.recipientAddress, req.transferAmount));

            // the function to calculate fee for a single entry is not exposed...
            const table = buildFeeTable(nymGasPrice(this.stakeDenom), {sendMultiple: nymGasLimits.send * data.length}, {sendMultiple: nymGasLimits.send * data.length})
            const fee = table.sendMultiple
            const result = await this.client.signAndBroadcast(senderAddress, encoded, fee, memo)
            if (isBroadcastTxFailure(result)) {
                throw new Error(`Error when broadcasting tx ${result.transactionHash} at height ${result.height}. Code: ${result.code}; Raw log: ${result.rawLog}`)
            }
            return result
        } else {
            throw new Error("Tried to use sendMultiple with a query client");
        }
    }

    public async executeCustom(signerAddress: string, messages: readonly EncodeObject[], customFee: StdFee, memo?: string): Promise<BroadcastTxSuccess> {
        if (this.client instanceof NetClient) {
            const result = await this.client.signAndBroadcast(signerAddress, messages, customFee, memo);
            if (isBroadcastTxFailure(result)) {
                throw new Error(`Error when broadcasting tx ${result.transactionHash} at height ${result.height}. Code: ${result.code}; Raw log: ${result.rawLog}`)
            }
            return result
        } else {
            throw new Error("Tried to use executeCustom with a query client");
        }
    }

    async upload(senderAddress: string, wasmCode: Uint8Array, meta?: UploadMeta, memo?: string): Promise<UploadResult> {
        if (this.client instanceof NetClient) {
            return this.client.upload(senderAddress, wasmCode, meta, memo).catch((err) => this.handleRequestFailure(err));
        } else {
            throw new Error("Tried to upload with a query client");
        }
    }

    public instantiate(senderAddress: string, codeId: number, initMsg: Record<string, unknown>, label: string, options?: InstantiateOptions): Promise<InstantiateResult> {
        if (this.client instanceof NetClient) {
            return this.client.instantiate(senderAddress, codeId, initMsg, label, options).catch((err) => this.handleRequestFailure(err));
        } else {
            throw new Error("Tried to instantiate with a query client");
        }
    }

    public migrate(senderAddress: string, contractAddress: string, codeId: number, migrateMsg: Record<string, unknown>, memo?: string): Promise<MigrateResult> {
        if (this.client instanceof NetClient) {
            return this.client.migrate(senderAddress, contractAddress, codeId, migrateMsg, memo).catch((err) => this.handleRequestFailure(err))
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
    epoch_length: number,
    // ideally I'd want to define those as `number` rather than `string`, but
    // rust-side they are defined as Uint128 and Decimal that don't have
    // native javascript representations and therefore are interpreted as strings after deserialization
    minimum_mixnode_bond: string,
    minimum_gateway_bond: string,
    mixnode_bond_reward_rate: string,
    gateway_bond_reward_rate: string,
    mixnode_active_set_size: number,
}
