import axios from "axios";
import { GasLimits } from "@cosmjs/launchpad";
import { CosmWasmFeeTable } from "@cosmjs/cosmwasm";


export interface Options {
    httpUrl: string;
    networkId: string;
    feeToken: string;
    gasPrice: number;
    bech32prefix: string;
}

export const nymGasLimits: GasLimits<CosmWasmFeeTable> = {
    upload: 2_500_000,
    init: 500_000,
    migrate: 200_000,
    exec: 9_000_000_000,
    send: 80_000,
    changeAdmin: 80_000,
};

export const defaultOptions: Options = {
    httpUrl: "http://localhost:26657",
    networkId: "nymnet",
    feeToken: "unym",
    gasPrice: 0.025,
    bech32prefix: "nym",
};

export const downloadWasm = async (url: string): Promise<Uint8Array> => {
    const r = await axios.get(url, { responseType: "arraybuffer" });
    if (r.status !== 200) {
        throw new Error(`Download error: ${r.status}`);
    }
    return r.data;
};

