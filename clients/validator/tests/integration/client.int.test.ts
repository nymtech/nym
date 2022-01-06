import validatorClient from "../../src/index";
import { config } from '../test-utils/config';

let client: validatorClient;
let mnemonic: string;

beforeEach(async () => {
    mnemonic = validatorClient.randomMnemonic();
    client = await validatorClient.connect(
        mnemonic,
        config.NYMD_URL as string,
        config.VALIDATOR_API as string,
        config.CURRENCY_PREFIX as string,
        config.MIXNET_CONTRACT as string,
        config.VESTING_CONTRACT as string
    );
});

//todos
//we want to mock the majority of these tests
//and keep a few integration tests in place

describe("connect to the nym validator client and perform integration tests against the current testnet", () => {
    test("get cached mixnodes", async () => {
        try {
            const response = await client.getCachedMixnodes();
            //expect all mixnodes to have their owner address
            response.forEach(mixnodeDetails => {
                expect(mixnodeDetails.owner).toHaveLength(43)
            });
        }
        catch (e) {
            console.log(e);
        }
    });

    test("get balance of address and denom of the network", async () => {
        try {
            //provide a users address and get their balance
            //we expect their balance to be zero, as it's a new account
            const address = await validatorClient.mnemonicToAddress(mnemonic, config.CURRENCY_PREFIX as string);
            const response = await client.getBalance(address);
            expect(response.amount).toStrictEqual("0");
            expect(response.denom).toBe("unymt");
        }
        catch (e) {
            console.log(e);
        }
    });

    test("get minimium pledge amount for a mixnode", async () => {
        try {
            const response = await client.minimumMixnodePledge();
            expect(response.amount).toBe("100000000");
            expect(response.denom).toBe(config.CURRENCY_PREFIX as string);
        }
        catch (e) {
            console.log(e);
        }
    });

    test("get minimium gateway pledge amount", async () => {
        try {
            const response = await client.minimumGatewayPledge();
            expect(response.amount).toBe("100000000");
            expect(response.denom).toBe(config.CURRENCY_PREFIX as string);
        }
        catch (e) {
            console.log(e);
        }
    });

    test("get current mixnet contract address", () => {
        try {
            const response = client.mixnetContract;
            expect(response).toStrictEqual(config.MIXNET_CONTRACT as string)
        }
        catch (e) {
            console.log(e);
        }
    });

    test("get current vesting contract address", () => {
        try {
            const response = client.vestingContract;
            expect(response).toStrictEqual(config.VESTING_CONTRACT as string)
        }
        catch (e) {
            console.log(e);
        }
    });
});
