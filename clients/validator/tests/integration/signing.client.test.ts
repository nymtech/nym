import SigningClient from '../../src/signing-client';
import validator from "../../src/index";
import { CosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import { DirectSecp256k1HdWallet } from '@cosmjs/proto-signing';
import { config } from '../test-utils/config';

let cosmWasmClient: CosmWasmClient;
let signingClient: SigningClient;
let mnemonic: string;

beforeEach(async () => {
    cosmWasmClient = await SigningClient.connect(config.NYMD_URL as string);
    
    mnemonic = validator.randomMnemonic();
    const wallet = await DirectSecp256k1HdWallet.fromMnemonic(mnemonic)
    signingClient = await SigningClient.connectWithNymSigner(
        wallet,
        config.NYMD_URL as string,
        config.VALIDATOR_API as string,
        config.CURRENCY_PREFIX as string);
});

describe("alternate between the signing clients of nym and perform basic requests", () => {
    test("retrieve balance using the cosmwasm client", async () => {
        try {
            const randomAddress = await validator.mnemonicToAddress(mnemonic, config.CURRENCY_PREFIX as string);
            const balance  = await cosmWasmClient.getBalance(randomAddress, config.CURRENCY_PREFIX as string);
            expect(balance.denom).toStrictEqual(config.CURRENCY_PREFIX as string);
            expect(balance.amount).toStrictEqual("0");
        }
        catch (e) {
            console.log(e);
        }
    });
    test("get the chain id of the network", async () => {
        try {
            //pass these values in environment variables in the future
            const chainId = await cosmWasmClient.getChainId();
            expect(chainId).toStrictEqual(config.CHAIN_ID as string);
        }
        catch (e) {
            console.log(e);
        }
    });
    test("get height then search its block", async () => {
        try {
            const height = await cosmWasmClient.getHeight()
            const block = await cosmWasmClient.getBlock(height);
            expect(block.header.chainId).toStrictEqual(config.CHAIN_ID as string);
            expect(block.header.height).toStrictEqual(height);
        }
        catch (e) {
            console.log(e);
        }
    });
    test("get current reward pool", async () => {
        try {
            //this is due to change due to when rewards get distributed
            const rewards = await signingClient.getRewardPool(config.MIXNET_CONTRACT as string);
            expect(rewards).toStrictEqual("250000000000000");
        }
        catch (e) {
            console.log(e);
        }
    });
});