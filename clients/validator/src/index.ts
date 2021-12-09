import SigningClient, {ISigningClient} from "./signing-client";
import {
    ContractSettingsParams,
    Delegation,
    Gateway,
    GatewayBond,
    MixNode,
    MixNodeBond,
    PagedGatewayDelegationsResponse,
    PagedMixDelegationsResponse,
    SendRequest
} from "./types";
import {Bip39, Random} from "@cosmjs/crypto";
import {DirectSecp256k1HdWallet, EncodeObject} from "@cosmjs/proto-signing";
import MixnodesCache from "./caches/mixnodes";
import {coin, Coin, coins, DeliverTxResponse, isDeliverTxFailure, StdFee} from "@cosmjs/stargate";
import {
    ExecuteResult,
    InstantiateOptions,
    InstantiateResult,
    MigrateResult,
    UploadResult
} from "@cosmjs/cosmwasm-stargate";
import {
    CoinMap,
    displayAmountToNative,
    MappedCoin,
    nativeCoinToDisplay,
    nativeToPrintable,
    printableBalance,
    printableCoin
} from "./currency";
import GatewaysCache from "./caches/gateways";
import QueryClient, {IQueryClient} from "./query-client";
import {nymGasPrice} from "./stargate-helper";
import {makeBankMsgSend} from "./utils";

export const VALIDATOR_API_PORT = "8080";
export const VALIDATOR_API_GATEWAYS = "v1/gateways";
export const VALIDATOR_API_MIXNODES = "v1/mixnodes";

export {coins, coin};
export {Coin};
export {
    displayAmountToNative,
    nativeCoinToDisplay,
    printableCoin,
    printableBalance,
    nativeToPrintable,
    MappedCoin,
    CoinMap
}
export {nymGasPrice}

export default class ValidatorClient {
    private readonly client: ISigningClient | IQueryClient
    private readonly contractAddress: string;
    private readonly denom: string;
    private gatewayCache: GatewaysCache
    private mixNodesCache: MixnodesCache;
    private readonly prefix: string;
    private url: string;


    private constructor(client: ISigningClient | IQueryClient, url: string, contractAddress: string, prefix: string) {
        this.client = client;
        this.mixNodesCache = new MixnodesCache(client, 100);
        this.gatewayCache = new GatewaysCache(client, 100);
        this.contractAddress = contractAddress;
        this.prefix = prefix;
        this.denom = "u" + prefix;
        this.url = url;
    }

    static async connect(contractAddress: string, mnemonic: string, url: string, prefix: string): Promise<ValidatorClient> {
        const wallet = await ValidatorClient.buildWallet(mnemonic, prefix);

        const netClient = await SigningClient.connect(wallet, url, prefix)
        return new ValidatorClient(netClient, url, contractAddress, prefix);
    }

    // allows also entering 'string' by itself for backwards compatibility
    static async connectForQuery(contractAddress: string, url: string, prefix: string): Promise<ValidatorClient> {
        const queryClient = await QueryClient.connect(url)
        return new ValidatorClient(queryClient, url, contractAddress, prefix)
    }

    public async changeValidator(newUrl: string): Promise<void> {
        console.log("Changing validator to", newUrl)
        return await this.client.changeValidator(newUrl)
    }

    public get address(): string {
        if (this.client instanceof SigningClient) {
            return this.client.clientAddress
        } else {
            return ""
        }
    }

    private assertSigning() {
        if (this.client instanceof QueryClient) {
            throw new Error("Tried to perform signing action with a query client!")
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
     * @param prefix the bech32 address prefix (human readable part)
     * @returns the address for this client wallet
     */
    static async mnemonicToAddress(mnemonic: string, prefix: string): Promise<string> {
        const wallet = await ValidatorClient.buildWallet(mnemonic, prefix);
        const [{address}] = await wallet.getAccounts()
        return address
    }

    static async buildWallet(mnemonic: string, prefix: string): Promise<DirectSecp256k1HdWallet> {
        const signerOptions = {prefix: prefix};
        return DirectSecp256k1HdWallet.fromMnemonic(mnemonic, signerOptions);
    }

    getBalance(address: string): Promise<Coin | null> {
        return this.client.getBalance(address, this.denom);
    }

    async getStateParams(): Promise<ContractSettingsParams> {
        return this.client.getContractSettingsParams(this.contractAddress)
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
     * Get or refresh the list of mixnodes in the network from validator-api
     *
     * @returns an array containing all known `MixNodeBond`s.
     *
     * TODO: We will want to put this puppy on a timer, but for the moment we can
     * just get things strung together and refresh it manually.
     */
    refreshValidatorAPIMixNodes(): Promise<MixNodeBond[]> {
        return this.mixNodesCache.refreshValidatorAPIMixNodes(this.url);
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
        this.assertSigning()
        const result = await this.client.execute(this.client.clientAddress, this.contractAddress, {bond_mixnode: {mix_node: mixNode}}, "auto", "adding mixnode", [bond]);
        console.log(`account ${this.client.clientAddress} added mixnode with ${mixNode.host}`);
        return result;


    }

    /**
     * Unbond a mixnode, removing it from the network and reclaiming staked coins
     */
    async unbondMixnode(): Promise<ExecuteResult> {
        if (this.client instanceof SigningClient) {
            const result = await this.client.execute(this.client.clientAddress, this.contractAddress, {unbond_mixnode: {}}, "auto")
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
        if (this.client instanceof SigningClient) {
            const result = await this.client.execute(this.client.clientAddress, this.contractAddress, {delegate_to_mixnode: {mix_identity: mixIdentity}}, "auto", `delegating to ${mixIdentity}`, [amount])
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
        if (this.client instanceof SigningClient) {
            const result = await this.client.execute(this.client.clientAddress, this.contractAddress, {undelegate_from_mixnode: {mix_identity: mixIdentity}}, "auto")
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
        if (this.client instanceof SigningClient) {
            const result = await this.client.execute(this.client.clientAddress, this.contractAddress, {delegate_to_gateway: {gateway_identity: gatewayIdentity}}, "auto", `delegating to ${gatewayIdentity}`, [amount])
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
        if (this.client instanceof SigningClient) {
            const result = await this.client.execute(this.client.clientAddress, this.contractAddress, {undelegate_from_gateway: {gateway_identity: gatewayIdentity}}, "auto",)
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
        if (this.client instanceof SigningClient) {
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
        if (this.client instanceof SigningClient) {
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
     * Get or refresh the list of gateways in the network from validator-api
     *
     * @returns an array containing all known `GatewayBond`s.
     *
     * TODO: Similarly to mixnode bonds, this should probably be put on a timer somewhere.
     */
    refreshValidatorAPIGateways(): Promise<GatewayBond[]> {
        return this.gatewayCache.refreshValidatorAPIGateways(this.url);
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
        if (this.client instanceof SigningClient) {
            const result = await this.client.execute(this.client.clientAddress, this.contractAddress, {bond_gateway: {gateway: gateway}}, "auto", "adding gateway", [bond]);
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
        if (this.client instanceof SigningClient) {
            const result = await this.client.execute(this.client.clientAddress, this.contractAddress, {unbond_gateway: {}}, "auto",)
            console.log(`account ${this.client.clientAddress} unbonded gateway`);
            return result;
        } else {
            throw new Error("Tried to unbond gateway with a query client")
        }
    }

    async updateStateParams(newParams: ContractSettingsParams): Promise<ExecuteResult> {
        if (this.client instanceof SigningClient) {
            return await this.client.execute(this.client.clientAddress, this.contractAddress, {update_contract_settings: newParams}, "auto", "updating contract settings");
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
    async send(senderAddress: string, recipientAddress: string, coins: readonly Coin[], memo?: string): Promise<DeliverTxResponse> {
        if (this.client instanceof SigningClient) {
            const result = await this.client.sendTokens(senderAddress, recipientAddress, coins, "auto", memo);
            if (isDeliverTxFailure(result)) {
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
    async sendMultiple(senderAddress: string, data: SendRequest[], memo?: string): Promise<DeliverTxResponse> {
        if (this.client instanceof SigningClient) {
            if (data.length === 1) {
                return this.send(data[0].senderAddress, data[0].recipientAddress, data[0].transferAmount, memo)
            }

            const encoded = data.map(req => makeBankMsgSend(req.senderAddress, req.recipientAddress, req.transferAmount));

            const result = await this.client.signAndBroadcast(senderAddress, encoded, "auto", memo)
            if (isDeliverTxFailure(result)) {
                throw new Error(`Error when broadcasting tx ${result.transactionHash} at height ${result.height}. Code: ${result.code}; Raw log: ${result.rawLog}`)
            }
            return result
        } else {
            throw new Error("Tried to use sendMultiple with a query client");
        }
    }

    public async executeCustom(signerAddress: string, messages: readonly EncodeObject[], customFee: StdFee, memo?: string): Promise<DeliverTxResponse> {
        if (this.client instanceof SigningClient) {
            const result = await this.client.signAndBroadcast(signerAddress, messages, customFee, memo);
            if (isDeliverTxFailure(result)) {
                throw new Error(`Error when broadcasting tx ${result.transactionHash} at height ${result.height}. Code: ${result.code}; Raw log: ${result.rawLog}`)
            }
            return result
        } else {
            throw new Error("Tried to use executeCustom with a query client");
        }
    }

    async upload(senderAddress: string, wasmCode: Uint8Array, memo?: string): Promise<UploadResult> {
        if (this.client instanceof SigningClient) {
            return this.client.upload(senderAddress, wasmCode, "auto", memo);
        } else {
            throw new Error("Tried to upload with a query client");
        }
    }

    public instantiate(senderAddress: string, codeId: number, initMsg: Record<string, unknown>, label: string, options?: InstantiateOptions): Promise<InstantiateResult> {
        if (this.client instanceof SigningClient) {
            return this.client.instantiate(senderAddress, codeId, initMsg, label, "auto", options);
        } else {
            throw new Error("Tried to instantiate with a query client");
        }
    }

    public migrate(senderAddress: string, contractAddress: string, codeId: number, migrateMsg: Record<string, unknown>, memo?: string): Promise<MigrateResult> {
        if (this.client instanceof SigningClient) {
            return this.client.migrate(senderAddress, contractAddress, codeId, migrateMsg, "auto", memo)
        } else {
            throw new Error("Tried to migrate with a query client");
        }
    }
}

