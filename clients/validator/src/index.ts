import NetClient, { INetClient, PagedResponse } from "./net-client";
import { MixNode, MixNodeBond } from "./types";
import * as fs from "fs";
import { Bip39, Random } from "@cosmjs/crypto";
import { DirectSecp256k1HdWallet } from "@cosmjs/proto-signing";
import MixnodesCache from "./caches/mixnodes";
import { Coin, coins } from "@cosmjs/launchpad";
import { BroadcastTxResponse } from "@cosmjs/stargate/types"
import { ExecuteResult, InstantiateOptions, InstantiateResult, UploadMeta, UploadResult } from "@cosmjs/cosmwasm";

export { coins };
export default class ValidatorClient {
    url: string;
    private netClient: INetClient;
    private mixNodesCache: MixnodesCache;
    private wallet: DirectSecp256k1HdWallet
    readonly address: string;
    private contractAddress: string;

    private constructor(url: string, netClient: INetClient, wallet: DirectSecp256k1HdWallet, address: string, contractAddress: string) {
        this.url = url;
        this.netClient = netClient;
        this.mixNodesCache = new MixnodesCache(netClient, 100);
        this.address = address;
        this.wallet = wallet;
        this.contractAddress = contractAddress;
    }

    static async connect(contractAddress: string, mnemonic: string, url: string,) {
        const wallet = await ValidatorClient.buildWallet(mnemonic);
        const [{ address }] = await wallet.getAccounts();
        const netClient = await NetClient.connect(contractAddress, wallet, url);
        return new ValidatorClient(url, netClient, wallet, address, contractAddress);
    }

    /**
     * Loads a named mnemonic from the system's keystore.
     * 
     * @param keyName the name of the key in the keystore
     * @returns the mnemonic as a string
     */
    static loadMnemonic(keyPath: string) {
        try {
            const mnemonic = fs.readFileSync(keyPath, "utf8");
            return mnemonic.trim();
        } catch (err) {
            console.log(err);
            return "fight with type system later";
        }
    }

    /**
     * Generates a random mnemonic, useful for creating new accounts.
     * @returns a fresh mnemonic.
     */
    static randomMnemonic(): string {
        const mnemonic = Bip39.encode(Random.getBytes(16)).toString();
        return mnemonic;
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
        return DirectSecp256k1HdWallet.fromMnemonic(mnemonic, undefined, "nym");
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
        const bond = [{ amount: "1000000000", denom: "unym" }];
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

