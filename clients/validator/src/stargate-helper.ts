import axios from "axios";
import { GasLimits, GasPrice } from "@cosmjs/launchpad";
import { CosmWasmFeeTable } from "@cosmjs/cosmwasm";


export interface Options {
    httpUrl: string;
    networkId: string;
    gasPrice: number;
    prefix: string;
}

export const nymGasLimits: GasLimits<CosmWasmFeeTable> = {
    upload: 2_500_000,
    init: 500_000,
    migrate: 200_000,
    exec: 250_000,
    send: 80_000,
    changeAdmin: 80_000,
};

export function nymGasPrice(prefix: string): GasPrice {
    return GasPrice.fromString(`0.025u${prefix}`); // TODO: ideally this ugly conversion shouldn't be hardcoded here.
};

export const defaultOptions: Options = {
    httpUrl: "http://localhost:26657",
    networkId: "nymnet",
    gasPrice: 0.025,
    prefix: "hal",
};

export const downloadWasm = async (url: string): Promise<Uint8Array> => {
    const r = await axios.get(url, { responseType: "arraybuffer" });
    if (r.status !== 200) {
        throw new Error(`Download error: ${r.status}`);
    }
    return r.data;
};

