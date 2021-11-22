import NetClient, { INetClient } from "./net-client";
import {
    ContractSettingsParams,
    Delegation,
    PagedMixDelegationsResponse,
    PagedGatewayDelegationsResponse,
    MixNodeBond,
    MixNode,
    GatewayBond,
    Gateway,
    SendRequest
} from "./types";
import { Bip39, Random } from "@cosmjs/crypto";
import { DirectSecp256k1HdWallet, EncodeObject } from "@cosmjs/proto-signing";
import MixnodesCache from "./caches/mixnodes";
import { buildFeeTable, coin, Coin, coins, StdFee } from "@cosmjs/stargate";
import {
    ExecuteResult,
    InstantiateOptions,
    InstantiateResult,
    MigrateResult,
    UploadMeta,
    UploadResult
} from "@cosmjs/cosmwasm-stargate";
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

export const VALIDATOR_API_PORT = "8080";
export const VALIDATOR_API_GATEWAYS = "v1/gateways";
export const VALIDATOR_API_MIXNODES = "v1/mixnodes";

export { coins, coin };
export { Coin };
export {
    displayAmountToNative,
    nativeCoinToDisplay,
    printableCoin,
    printableBalance,
    nativeToPrintable,
    MappedCoin,
    CoinMap
}
export { nymGasLimits, nymGasPrice }

export default class ValidatorClient {
    private readonly client: INetClient | IQueryClient
    private readonly contractAddress: string;
    private readonly denom: string;
    private failedRequests: number = 0;
    private gatewayCache: GatewaysCache
    private mixNodesCache: MixnodesCache;
    private readonly prefix: string;
    urls: string[];


    private constructor(urls: string[], client: INetClient | IQueryClient, contractAddress: string, prefix: string) {
        this.urls = urls;
        this.client = client;
        this.mixNodesCache = new MixnodesCache(client, 100);
        this.gatewayCache = new GatewaysCache(client, 100);
        this.contractAddress = contractAddress;
        this.prefix = prefix;
        this.denom = "u" + prefix;
    }

    // allows also entering 'string' by itself for backwards compatibility
    static async connect(contractAddress: string, mnemonic: string, urls: string | string[], prefix: string): Promise<ValidatorClient> {
        const validatorUrls = this.ensureArray(urls)
        const wallet = await ValidatorClient.buildWallet(mnemonic, prefix);

        // if we have more than a single validator, try to perform initial connection until we succeed or run out of options
        if (validatorUrls.length > 1) {
            for (let i = 0; i < validatorUrls.length; i++) {
                console.log("Attempting initial connection to", validatorUrls[0])
                const netClient = await NetClient.connect(wallet, validatorUrls[0], prefix).catch((_) => ValidatorClient.moveArrayHeadToBack(validatorUrls))
                if (netClient !== undefined) {
                    return new ValidatorClient(validatorUrls, netClient, contractAddress, prefix);
                }
                console.log("Initial connection to", validatorUrls[0], "failed")
            }
        } else {
            const netClient = await NetClient.connect(wallet, validatorUrls[0], prefix)
            return new ValidatorClient(validatorUrls, netClient, contractAddress, prefix);
        }

        throw new Error("None of the provided validators seem to be alive")
    }

    // allows also entering 'string' by itself for backwards compatibility
    static async connectForQuery(contractAddress: string, urls: string | string[], prefix: string): Promise<ValidatorClient> {
        const validatorUrls = this.ensureArray(urls)

        // if we have more than a single validator, try to perform initial connection until we succeed or run out of options
        if (validatorUrls.length > 1) {
            for (let i = 0; i < validatorUrls.length; i++) {
                console.log("Attempting initial connection to", validatorUrls[0])
                const queryClient = await QueryClient.connect(validatorUrls[0]).catch((_) => ValidatorClient.moveArrayHeadToBack(validatorUrls))
                if (queryClient !== undefined) {
                    return new ValidatorClient(validatorUrls, queryClient, contractAddress, prefix)
                }
                console.log("Initial connection to", validatorUrls[0], "failed")
            }
        } else {
            const queryClient = await QueryClient.connect(validatorUrls[0])
            return new ValidatorClient(validatorUrls, queryClient, contractAddress, prefix)
        }

        throw new Error("None of the provided validators seem to be alive")
    }

    private static ensureArray(urls: string | string[]): string[] {
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
            return await this.changeValidator(this.urls[0]).then(() => {
                throw error
            })
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
        const head = <T>arr.shift()
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
    static async mnemonicToAddress(mnemonic: string, prefix: string): Promise<string> {
        const wallet = await ValidatorClient.buildWallet(mnemonic, prefix);
        const [{ address }] = await wallet.getAccounts()
        return address
    }

    static async buildWallet(mnemonic: string, prefix: string): Promise<DirectSecp256k1HdWallet> {
        const signerOptions = { prefix: prefix };
        return DirectSecp256k1HdWallet.fromMnemonic(mnemonic, signerOptions);
    }

    getBalance(address: string): Promise<Coin | null> {
        return this.client.getBalance(address, this.denom).catch((err) => this.handleRequestFailure(err));
    }

    async getStateParams(): Promise<ContractSettingsParams> {
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
     * Get or refresh the list of mixnodes in the network from validator-api
     *
     * @returns an array containing all known `MixNodeBond`s.
     *
     * TODO: We will want to put this puppy on a timer, but for the moment we can
     * just get things strung together and refresh it manually.
     */
    refreshValidatorAPIMixNodes(): Promise<MixNodeBond[]> {
        return this.mixNodesCache.refreshValidatorAPIMixNodes(this.urls).catch((err) => this.handleRequestFailure(err));
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
    async minimumMixnodeBond(): Promise<Coin> {
        const stateParams = await this.getStateParams()
        // we trust the contract to return a valid number
        return coin(Number(stateParams.minimum_mixnode_bond), this.prefix)
    }

    /**
     *  Announce a mixnode, paying a fee.
     */
    async bondMixnode(mixNode: MixNode, bond: Coin): Promise<ExecuteResult> {
        if (this.client instanceof NetClient) {
            const result = await this.client.executeContract(this.client.clientAddress, this.contractAddress, { bond_mixnode: { mix_node: mixNode } }, "adding mixnode", [bond]).catch((err) => this.handleRequestFailure(err));
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
     * Delegates specified amount of stake to particular mixnode.
     *
     * @param mixIdentity identity of the node to which the delegation should be applied
     * @param amount desired amount of coins to delegate to the node
     */
    // requires coin type to ensure correct denomination (
    async delegateToMixnode(mixIdentity: string, amount: Coin): Promise<ExecuteResult> {
        if (this.client instanceof NetClient) {
            const result = await this.client.executeContract(this.client.clientAddress, this.contractAddress, { delegate_to_mixnode: { mix_identity: mixIdentity } }, `delegating to ${mixIdentity}`, [amount]).catch((err) => this.handleRequestFailure(err))
            console.log(`account ${this.client.clientAddress} delegated ${amount} to mixnode ${mixIdentity}`);
            return result;
        } else {
            throw new Error("Tried to delegate stake with a query client")
        }
    }

    /**
     * Removes stake delegation from a particular mixnode.
     *
     * @param mixIdentity identity of the node from which the delegation should get removed
     */
    async removeMixnodeDelegation(mixIdentity: string): Promise<ExecuteResult> {
        if (this.client instanceof NetClient) {
            const result = await this.client.executeContract(this.client.clientAddress, this.contractAddress, { undelegate_from_mixnode: { mix_identity: mixIdentity } }).catch((err) => this.handleRequestFailure(err))
            console.log(`account ${this.client.clientAddress} removed delegation from mixnode ${mixIdentity}`);
            return result;
        } else {
            throw new Error("Tried to remove stake delegation with a query client")
        }
    }

    /**
     * Delegates specified amount of stake to particular gateway.
     *
     * @param gatewayIdentity identity of the gateway to which the delegation should be applied
     * @param amount desired amount of coins to delegate to the node
     */
    // requires coin type to ensure correct denomination (
    async delegateToGateway(gatewayIdentity: string, amount: Coin): Promise<ExecuteResult> {
        if (this.client instanceof NetClient) {
            const result = await this.client.executeContract(this.client.clientAddress, this.contractAddress, { delegate_to_gateway: { gateway_identity: gatewayIdentity } }, `delegating to ${gatewayIdentity}`, [amount]).catch((err) => this.handleRequestFailure(err))
            console.log(`account ${this.client.clientAddress} delegated ${amount} to gateway ${gatewayIdentity}`);
            return result;
        } else {
            throw new Error("Tried to delegate stake with a query client")
        }
    }

    /**
     * Removes stake delegation from a particular gateway.
     *
     * @param gatewayIdentity identity of the gateway from which the delegation should get removed
     */
    async removeGatewayDelegation(gatewayIdentity: string): Promise<ExecuteResult> {
        if (this.client instanceof NetClient) {
            const result = await this.client.executeContract(this.client.clientAddress, this.contractAddress, { undelegate_from_gateway: { gateway_identity: gatewayIdentity } }).catch((err) => this.handleRequestFailure(err))
            console.log(`account ${this.client.clientAddress} removed delegation from gateway ${gatewayIdentity}`);
            return result;
        } else {
            throw new Error("Tried to remove stake delegation with a query client")
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
     * Get or refresh the list of gateways in the network from validator-api
     *
     * @returns an array containing all known `GatewayBond`s.
     *
     * TODO: Similarly to mixnode bonds, this should probably be put on a timer somewhere.
     */
    refreshValidatorAPIGateways(): Promise<GatewayBond[]> {
        return this.gatewayCache.refreshValidatorAPIGateways(this.urls).catch((err) => this.handleRequestFailure(err));
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
    async minimumGatewayBond(): Promise<Coin> {
        const stateParams = await this.getStateParams()
        // we trust the contract to return a valid number
        return coin(Number(stateParams.minimum_gateway_bond), this.prefix)
    }

    /**
     *  Announce a gateway, paying a fee.
     */
    async bondGateway(gateway: Gateway, bond: Coin): Promise<ExecuteResult> {
        if (this.client instanceof NetClient) {
            const result = await this.client.executeContract(this.client.clientAddress, this.contractAddress, { bond_gateway: { gateway: gateway } }, "adding gateway", [bond]).catch((err) => this.handleRequestFailure(err));
            console.log(`account ${this.client.clientAddress} added gateway with ${gateway.host}`);
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
            const result = await this.client.executeContract(this.client.clientAddress, this.contractAddress, { unbond_gateway: {} }).catch((err) => this.handleRequestFailure(err))
            console.log(`account ${this.client.clientAddress} unbonded gateway`);
            return result;
        } else {
            throw new Error("Tried to unbond gateway with a query client")
        }
    }

    async updateStateParams(newParams: ContractSettingsParams): Promise<ExecuteResult> {
        if (this.client instanceof NetClient) {
            return await this.client.executeContract(this.client.clientAddress, this.contractAddress, { update_contract_settings: newParams }, "updating contract settings").catch((err) => this.handleRequestFailure(err));
        } else {
            throw new Error("Tried to update state params with a query client")
        }
    }

    /**
     * Gets list of all delegations towards particular mixnode.
     *
     * @param mixIdentity identity of the node to which the delegation was sent
     */
    public async getMixDelegations(mixIdentity: string): Promise<Delegation[]> {
        // make this configurable somewhere
        const limit = 500

        let delegations: Delegation[] = [];
        let response: PagedMixDelegationsResponse
        let next: string | undefined = undefined;
        for (; ;) {
            response = await this.client.getMixDelegations(this.contractAddress, mixIdentity, limit, next)
            delegations = delegations.concat(response.delegations)
            next = response.start_next_after
            // if `start_next_after` is not set, we're done
            if (!next) {
                break
            }
        }

        return delegations
    }

    /**
     * Checks value of delegation of given client towards particular mixnode.
     *
     * @param mixIdentity identity of the node to which the delegation was sent
     * @param delegatorAddress address of the client who delegated the stake
     */
    public getMixDelegation(mixIdentity: string, delegatorAddress: string): Promise<Delegation> {
        return this.client.getMixDelegation(this.contractAddress, mixIdentity, delegatorAddress);
    }

    /**
     * Gets list of all delegations towards particular gateway.
     *
     * @param gatewayIdentity identity of the gateway to which the delegation was sent
     */
    public async getGatewayDelegations(gatewayIdentity: string): Promise<Delegation[]> {
        // make this configurable somewhere
        const limit = 500

        let delegations: Delegation[] = [];
        let response: PagedGatewayDelegationsResponse
        let next: string | undefined = undefined;
        for (; ;) {
            response = await this.client.getGatewayDelegations(this.contractAddress, gatewayIdentity, limit, next)
            delegations = delegations.concat(response.delegations)
            next = response.start_next_after
            // if `start_next_after` is not set, we're done
            if (!next) {
                break
            }
        }

        return delegations
    }

    /**
     * Checks value of delegation of given client towards particular gateway.
     *
     * @param gatewayIdentity identity of the gateway to which the delegation was sent
     * @param delegatorAddress address of the client who delegated the stake
     */
    public getGatewayDelegation(gatewayIdentity: string, delegatorAddress: string): Promise<Delegation> {
        return this.client.getGatewayDelegation(this.contractAddress, gatewayIdentity, delegatorAddress);
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
            console.log(`this.denom is ${this.denom}`);
            const table = buildFeeTable(nymGasPrice(this.prefix), { sendMultiple: nymGasLimits.send * data.length }, { sendMultiple: nymGasLimits.send * data.length })
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

