import { SigningCosmWasmClient, SigningCosmWasmClientOptions } from "@cosmjs/cosmwasm-stargate";
import { Bip39, Random } from "@cosmjs/crypto";
import { DirectSecp256k1HdWallet } from "@cosmjs/proto-signing";
import * as fs from "fs";
import axios from 'axios';
import { GasLimits, GasPrice, logs } from "@cosmjs/launchpad";
import { CosmWasmFeeTable } from "@cosmjs/cosmwasm";

export { connect, getAttribute, loadMnemonic, randomMnemonic, mnemonicToAddress };

interface Options {
    httpUrl: string;
    networkId: string;
    feeToken: string;
    gasPrice: number;
    bech32prefix: string;
}

const nymGasLimits: GasLimits<CosmWasmFeeTable> = {
    upload: 2_500_000,
    init: 500_000,
    migrate: 200_000,
    exec: 900000_000,
    send: 80_000,
    changeAdmin: 80_000,
};

const defaultOptions: Options = {
    httpUrl: "http://localhost:26657",
    networkId: "nymnet",
    feeToken: "unym",
    gasPrice: 0.025,
    bech32prefix: "nym",
};

const connect = async (
    mnemonic: string,
    opts: Partial<Options>
): Promise<{
    client: SigningCosmWasmClient
    address: string
}> => {
    const options: Options = { ...defaultOptions, ...opts }
    const wallet = await buildWallet(mnemonic);
    const [{ address }] = await wallet.getAccounts();
    const signerOptions: SigningCosmWasmClientOptions = {
        gasPrice: GasPrice.fromString("0.025unym"),
        gasLimits: nymGasLimits,
    };
    const client = await SigningCosmWasmClient.connectWithSigner(options.httpUrl, wallet, signerOptions);
    return { client, address }
}

const loadMnemonic = (keyPath: string): string => {
    try {
        const mnemonic = fs.readFileSync(keyPath, "utf8");
        return mnemonic.trim();
    } catch (err) {
        console.log(err);
        return "fight with type system later";
    }
};

const buildWallet = (mnemonic: string): Promise<DirectSecp256k1HdWallet> => {
    return DirectSecp256k1HdWallet.fromMnemonic(mnemonic, undefined, defaultOptions.bech32prefix);
}



const randomMnemonic = async (): Promise<string> => {
    const mnemonic = Bip39.encode(Random.getBytes(16)).toString();
    return mnemonic;
}

const mnemonicToAddress = async (mnemonic: string): Promise<string> => {
    const wallet = await buildWallet(mnemonic);
    const [{ address }] = await wallet.getAccounts()
    return address
}

const downloadWasm = async (url: string): Promise<Uint8Array> => {
    const r = await axios.get(url, { responseType: "arraybuffer" });
    if (r.status !== 200) {
        throw new Error(`Download error: ${r.status}`);
    }
    return r.data;
};

const getAttribute = (
    logs: readonly logs.Log[],
    key: string
): string | undefined =>
    logs[0].events[0].attributes.find((x) => x.key == key)?.value