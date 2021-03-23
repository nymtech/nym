import NetClient, { INetClient } from "./net-client";
import { Gateway, GatewayBond, MixNode, MixNodeBond } from "./types";
// import * as fs from "fs";
import { Bip39, Random } from "@cosmjs/crypto";
import { DirectSecp256k1HdWallet } from "@cosmjs/proto-signing";
import MixnodesCache from "./caches/mixnodes";
import { coin, Coin, coins } from "@cosmjs/launchpad";
import { BroadcastTxResponse } from "@cosmjs/stargate"
import { ExecuteResult, InstantiateOptions, InstantiateResult, UploadMeta, UploadResult } from "@cosmjs/cosmwasm";
import { CoinMap, displayAmountToNative, MappedCoin, nativeCoinToDisplay, printableBalance, printableCoin } from "./currency";
import GatewaysCache from "./caches/gateways";

export { coins };
export { Coin };
export { displayAmountToNative, nativeCoinToDisplay, printableCoin, printableBalance, MappedCoin, CoinMap }

export default class ValidatorClient {

    private readonly stakeDenom: string;
    private readonly gatewayBondingStake: number = 1000_000000

    url: string;
    private netClient: INetClient;
    private mixNodesCache: MixnodesCache;
    private gatewayCache: GatewaysCache
    private wallet: DirectSecp256k1HdWallet
    readonly address: string;
    private readonly contractAddress: string;

    private constructor(url: string, netClient: INetClient, wallet: DirectSecp256k1HdWallet, address: string, contractAddress: string, stakeDenom: string) {
        this.url = url;
        this.netClient = netClient;
        this.mixNodesCache = new MixnodesCache(netClient, 100);
        this.gatewayCache = new GatewaysCache(netClient, 100);
        this.address = address;
        this.wallet = wallet;
        this.contractAddress = contractAddress;
        this.stakeDenom = stakeDenom;
    }

    static async connect(contractAddress: string, mnemonic: string, url: string, stakeDenom: string): Promise<ValidatorClient> {
        const wallet = await ValidatorClient.buildWallet(mnemonic);
        const [{ address }] = await wallet.getAccounts();
        const netClient = await NetClient.connect(wallet, url, stakeDenom);
        return new ValidatorClient(url, netClient, wallet, address, contractAddress, stakeDenom);
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
        return Bip39.encode(Random.getBytes(16)).toString();
    }

    /**
     * @param mnemonic A mnemonic from which to generate a public/private keypair.
     * @returns the address for this client wallet
     */
    async mnemonicToAddress(mnemonic: string): Promise<string> {
        const wallet = await ValidatorClient.buildWallet(mnemonic);
        const [{ address }] = await wallet.getAccounts()
        return address
    }

    static async buildWallet(mnemonic: string): Promise<DirectSecp256k1HdWallet> {
        return DirectSecp256k1HdWallet.fromMnemonic(mnemonic, undefined, "hal");
    }

    getBalance(address: string): Promise<Coin | null> {
        return this.netClient.getBalance(address);
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
        const bond = [{ amount: "1000000000", denom: this.stakeDenom }];
        const result = await this.netClient.executeContract(this.address, this.contractAddress, { register_mixnode: { mix_node: mixNode } }, "adding mixnode", bond);
        console.log(`account ${this.address} added mixnode with ${mixNode.host}`);
        return result;
    }

    /**
     * Unbond a mixnode, removing it from the network and reclaiming staked coins
     */
    async unbond(): Promise<ExecuteResult> {
        const result = await this.netClient.executeContract(this.address, this.contractAddress, { un_register_mixnode: {} })
        console.log(`account ${this.address} unbonded mixnode`);
        return result;
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
        const bond = this.minimumGatewayBond()
        const result = await this.netClient.executeContract(this.address, this.contractAddress, { bond_gateway: { gateway: gateway } }, "adding gateway", [bond]);
        console.log(`account ${this.address} added gateway with ${gateway.mix_host}`);
        return result;
    }

    /**
     * Unbond a gateway, removing it from the network and reclaiming staked coins
     */
    async unbondGateway(): Promise<ExecuteResult> {
        const result = await this.netClient.executeContract(this.address, this.contractAddress, { unbond_gateway: {} })
        console.log(`account ${this.address} unbonded gateway`);
        return result;
    }


    // TODO: if we just keep a reference to the SigningCosmWasmClient somewhere we can probably go direct
    // to it in the case of these methods below.

    /**
     * Send funds from one address to another.
     */
    async send(senderAddress: string, recipientAddress: string, coins: readonly Coin[], memo?: string): Promise<BroadcastTxResponse> {
        return this.netClient.sendTokens(senderAddress, recipientAddress, coins, memo);
    }

    async upload(senderAddress: string, wasmCode: Uint8Array, meta?: UploadMeta, memo?: string): Promise<UploadResult> {
        return this.netClient.upload(senderAddress, wasmCode, meta, memo);
    }

    public instantiate(senderAddress: string, codeId: number, initMsg: Record<string, unknown>, label: string, options?: InstantiateOptions): Promise<InstantiateResult> {
        return this.netClient.instantiate(senderAddress, codeId, initMsg, label, options);
    }
}

