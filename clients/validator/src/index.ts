import SigningClient from "./signing-client";
import {
    ContractStateParams,
    Delegation,
    GatewayBond,
    MixnetContractVersion,
    MixNodeBond,
    PagedAllDelegationsResponse,
    PagedDelegatorDelegationsResponse,
    PagedGatewayResponse,
    PagedMixDelegationsResponse,
    PagedMixnodeResponse,
} from "./types";
import {Bip39, Random} from "@cosmjs/crypto";
import {DirectSecp256k1HdWallet} from "@cosmjs/proto-signing";
import {coin, Coin, coins} from "@cosmjs/stargate";
import {
    CoinMap,
    displayAmountToNative,
    MappedCoin,
    nativeCoinToDisplay,
    nativeToPrintable,
    printableBalance,
    printableCoin
} from "./currency";
import QueryClient from "./query-client";
import {nymGasPrice} from "./stargate-helper";

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


export interface INymClient {
    readonly mixnetContract: string,
    readonly vestingContract: string,
}

export default class ValidatorClient implements INymClient {
    readonly client: SigningClient | QueryClient
    private readonly denom: string;
    private readonly prefix: string;
    private nymdUrl: string;
    validatorApiUrl: string;

    readonly mixnetContract: string;
    readonly vestingContract: string;


    private constructor(
        client: SigningClient | QueryClient,
        nymdUrl: string,
        validatorApiUrl: string,
        prefix: string,
        mixnetContract: string,
        vestingContract: string
    ) {
        this.client = client;
        this.prefix = prefix;
        this.denom = "u" + prefix;
        this.nymdUrl = nymdUrl;
        this.validatorApiUrl = validatorApiUrl;

        this.mixnetContract = mixnetContract;
        this.vestingContract = vestingContract;
    }

    static async connect(
        mnemonic: string,
        nymdUrl: string,
        validatorApiUrl: string,
        prefix: string,
        mixnetContract: string,
        vestingContract: string,
    ): Promise<ValidatorClient> {
        const wallet = await ValidatorClient.buildWallet(mnemonic, prefix);

        const netClient = await SigningClient.connectWithNymSigner(wallet, nymdUrl, validatorApiUrl, prefix)
        return new ValidatorClient(netClient, nymdUrl, validatorApiUrl, prefix, mixnetContract, vestingContract);
    }

    // allows also entering 'string' by itself for backwards compatibility
    static async connectForQuery(
        contractAddress: string,
        nymdUrl: string,
        validatorApiUrl: string,
        prefix: string,
        mixnetContract: string,
        vestingContract: string
    ): Promise<ValidatorClient> {
        const queryClient = await QueryClient.connectWithNym(nymdUrl, validatorApiUrl)
        return new ValidatorClient(queryClient, nymdUrl, validatorApiUrl, prefix, mixnetContract, vestingContract)
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


    getBalance(address: string): Promise<Coin> {
        return this.client.getBalance(address, this.denom);
    }

    async getCachedGateways(): Promise<GatewayBond[]> {
        return this.client.getCachedGateways()
    }

    async getCachedMixnodes(): Promise<MixNodeBond[]> {
        return this.client.getCachedMixnodes()
    }

    async getActiveMixnodes(): Promise<MixNodeBond[]> {
        return this.client.getActiveMixnodes()
    }

    async getRewardedMixnodes(): Promise<MixNodeBond[]> {
        return this.client.getRewardedMixnodes()
    }

    public async getMixnetContractSettings(): Promise<ContractStateParams> {
        return this.client.getStateParams(this.mixnetContract)
    }

    public async getMixnetContractVersion(): Promise<MixnetContractVersion> {
        return this.client.getContractVersion(this.mixnetContract)
    }

    public async getRewardPool(): Promise<string> {
        return this.client.getRewardPool(this.mixnetContract)
    }

    public async getCirculatingSupply(): Promise<string> {
        return this.client.getCirculatingSupply(this.mixnetContract)
    }

    public async getSybilResistancePercent(): Promise<number> {
        return this.client.getSybilResistancePercent(this.mixnetContract)
    }

    public async getEpochRewardPercent(): Promise<number> {
        return this.client.getEpochRewardPercent(this.mixnetContract)
    }

    public async getAllNymdMixnodes(): Promise<MixNodeBond[]> {
        let mixNodes: MixNodeBond[] = [];
        const limit = 50;
        let startAfter = undefined;
        for (; ;) {
            const pagedResponse: PagedMixnodeResponse = await this.client.getMixNodesPaged(this.mixnetContract, limit, startAfter)
            mixNodes = mixNodes.concat(pagedResponse.nodes)
            startAfter = pagedResponse.start_next_after
            // if `start_next_after` is not set, we're done
            if (!startAfter) {
                break
            }
        }

        return mixNodes
    }

    public async getAllNymdGateways(): Promise<GatewayBond[]> {
        let gateways: GatewayBond[] = [];
        const limit = 50;
        let startAfter = undefined;
        for (; ;) {
            const pagedResponse: PagedGatewayResponse = await this.client.getGatewaysPaged(this.mixnetContract, limit, startAfter)
            gateways = gateways.concat(pagedResponse.nodes)
            startAfter = pagedResponse.start_next_after
            // if `start_next_after` is not set, we're done
            if (!startAfter) {
                break
            }
        }

        return gateways
    }


    /**
     * Gets list of all delegations towards particular mixnode.
     *
     * @param mixIdentity identity of the node to which the delegation was sent
     */
    public async getAllNymdSingleMixnodeDelegations(mixIdentity: string): Promise<Delegation[]> {
        let delegations: Delegation[] = [];
        const limit = 250;
        let startAfter = undefined;
        for (; ;) {
            const pagedResponse: PagedMixDelegationsResponse = await this.client.getMixNodeDelegationsPaged(this.mixnetContract, mixIdentity, limit, startAfter)
            delegations = delegations.concat(pagedResponse.delegations)
            startAfter = pagedResponse.start_next_after
            // if `start_next_after` is not set, we're done
            if (!startAfter) {
                break
            }
        }

        return delegations
    }

    public async getAllNymdDelegatorDelegations(delegator: string): Promise<Delegation[]> {
        let delegations: Delegation[] = [];
        const limit = 250;
        let startAfter = undefined;
        for (; ;) {
            const pagedResponse: PagedDelegatorDelegationsResponse = await this.client.getDelegatorDelegationsPaged(this.mixnetContract, delegator, limit, startAfter)
            delegations = delegations.concat(pagedResponse.delegations)
            startAfter = pagedResponse.start_next_after
            // if `start_next_after` is not set, we're done
            if (!startAfter) {
                break
            }
        }

        return delegations
    }

    public async getAllNymdNetworkDelegations(): Promise<Delegation[]> {
        let delegations: Delegation[] = [];
        const limit = 250;
        let startAfter = undefined;
        for (; ;) {
            const pagedResponse: PagedAllDelegationsResponse = await this.client.getAllNetworkDelegationsPaged(this.mixnetContract, limit, startAfter)
            delegations = delegations.concat(pagedResponse.delegations)
            startAfter = pagedResponse.start_next_after
            // if `start_next_after` is not set, we're done
            if (!startAfter) {
                break
            }
        }

        return delegations
    }


    /**
     * Generate a minimum gateway bond required to create a fresh mixnode.
     *
     * @returns a `Coin` instance containing minimum amount of coins to pledge a gateway.
     */
    public async minimumMixnodePledge(): Promise<Coin> {
        const stateParams = await this.getMixnetContractSettings()
        // we trust the contract to return a valid number
        return coin(stateParams.minimum_mixnode_pledge, this.prefix)
    }


    /**
     * Generate a minimum gateway bond required to create a fresh gateway.
     *
     * @returns a `Coin` instance containing minimum amount of coins to pledge a gateway.
     */
    public async minimumGatewayPledge(): Promise<Coin> {
        const stateParams = await this.getMixnetContractSettings()
        // we trust the contract to return a valid number
        return coin(stateParams.minimum_gateway_pledge, this.prefix)
    }


    // // TODO: if we just keep a reference to the SigningCosmWasmClient somewhere we can probably go direct
    // // to it in the case of these methods below.
    //
    // /**
    //  * Send funds from one address to another.
    //  */
    // async send(senderAddress: string, recipientAddress: string, coins: readonly Coin[], memo?: string): Promise<DeliverTxResponse> {
    //     this.assertSigning()
    //
    //     const result = await (this.client as ISigningClient).sendTokens(senderAddress, recipientAddress, coins, "auto", memo);
    //     if (isDeliverTxFailure(result)) {
    //         throw new Error(`Error when broadcasting tx ${result.transactionHash} at height ${result.height}. Code: ${result.code}; Raw log: ${result.rawLog}`)
    //     }
    //     return result
    //
    // }
    //
    // /**
    //  * Send funds multiple times from one address to another in a single block.
    //  */
    // async sendMultiple(senderAddress: string, data: SendRequest[], memo?: string): Promise<DeliverTxResponse> {
    //     this.assertSigning()
    //
    //     if (data.length === 1) {
    //         return this.send(data[0].senderAddress, data[0].recipientAddress, data[0].transferAmount, memo)
    //     }
    //
    //     const encoded = data.map(req => makeBankMsgSend(req.senderAddress, req.recipientAddress, req.transferAmount));
    //
    //     const result = await (this.client as ISigningClient).signAndBroadcast(senderAddress, encoded, "auto", memo)
    //     if (isDeliverTxFailure(result)) {
    //         throw new Error(`Error when broadcasting tx ${result.transactionHash} at height ${result.height}. Code: ${result.code}; Raw log: ${result.rawLog}`)
    //     }
    //     return result
    //
    // }
    //
    // public async executeCustom(signerAddress: string, messages: readonly EncodeObject[], customFee: StdFee, memo?: string): Promise<DeliverTxResponse> {
    //     this.assertSigning()
    //
    //     const result = await (this.client as ISigningClient).signAndBroadcast(signerAddress, messages, customFee, memo);
    //     if (isDeliverTxFailure(result)) {
    //         throw new Error(`Error when broadcasting tx ${result.transactionHash} at height ${result.height}. Code: ${result.code}; Raw log: ${result.rawLog}`)
    //     }
    //     return result
    //
    // }
    //
    // async upload(senderAddress: string, wasmCode: Uint8Array, memo?: string): Promise<UploadResult> {
    //     this.assertSigning()
    //     return (this.client as ISigningClient).upload(senderAddress, wasmCode, "auto", memo);
    //
    // }
    //
    // public instantiate(senderAddress: string, codeId: number, initMsg: Record<string, unknown>, label: string, options?: InstantiateOptions): Promise<InstantiateResult> {
    //     this.assertSigning()
    //     return (this.client as ISigningClient).instantiate(senderAddress, codeId, initMsg, label, "auto", options);
    //
    // }
    //
    // public migrate(senderAddress: string, contractAddress: string, codeId: number, migrateMsg: Record<string, unknown>, memo?: string): Promise<MigrateResult> {
    //     this.assertSigning()
    //     return (this.client as ISigningClient).migrate(senderAddress, contractAddress, codeId, migrateMsg, "auto", memo)
    // }
}

