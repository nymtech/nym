import SigningClient from '../../src/signing-client';
import validator from "../../src/index";
import { CosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import { config } from '../test-utils/config';

let cosmWasmClient: CosmWasmClient;
let mnemonic: string;

beforeEach(async () => {
    cosmWasmClient = await SigningClient.connect(config.NYMD_URL);
});

describe("peform basic network checks with the cosmwasm client", () => {
    test("retrieve a newly created users balance", async () => {
        try {
            const randomAddress = await validator.mnemonicToAddress(mnemonic, config.CURRENCY_PREFIX);
            const balance  = await cosmWasmClient.getBalance(randomAddress, config.CURRENCY_PREFIX);

            expect(balance.denom).toStrictEqual(config.CURRENCY_PREFIX);
            expect(balance.amount).toStrictEqual("0");
        }
        catch (error) {
            throw error;
        }
    });

    test("get the chain id of the network", async () => {
        try {
            const chainId = await cosmWasmClient.getChainId();
            expect(chainId).toStrictEqual(config.CHAIN_ID);
        }
        catch (error) {
            throw error;
        }
    });

    test("get height then search for it by it's block", async () => {
        try {
            const height = await cosmWasmClient.getHeight()
            const block = await cosmWasmClient.getBlock(height);

            expect(block.header.chainId).toStrictEqual(config.CHAIN_ID);
            expect(block.header.height).toStrictEqual(height);
        }
        catch (error) {
            throw error;
        }
    });
});