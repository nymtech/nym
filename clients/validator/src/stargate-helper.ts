import axios from "axios";
import { GasLimits, GasPrice } from "@cosmjs/stargate";
import { CosmWasmFeeTable, defaultGasLimits } from "@cosmjs/cosmwasm-stargate";

export const nymGasLimits: GasLimits<CosmWasmFeeTable> = {
    ...defaultGasLimits,
    upload: 2_500_000,
    init: 500_000,
    migrate: 200_000,
    exec: 250_000,
    send: 80_000,
    changeAdmin: 80_000,
};

export function nymGasPrice(prefix: string): GasPrice {
    return GasPrice.fromString(`0.025u${prefix}`); // TODO: ideally this ugly conversion shouldn't be hardcoded here.
}

export const downloadWasm = async (url: string): Promise<Uint8Array> => {
    const r = await axios.get(url, {responseType: "arraybuffer"});
    if (r.status !== 200) {
        throw new Error(`Download error: ${r.status}`);
    }
    return r.data;
};

