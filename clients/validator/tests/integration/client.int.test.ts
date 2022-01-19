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

describe("perform a few non expensive network calls with the validator client", () => {
    test("get all cached mixnodes", async () => {
        try {
            const response = await client.getCachedMixnodes();

            //expect all mixnodes to have their owner address
            response.forEach(mixnodeDetails => {
                expect(mixnodeDetails.owner).toHaveLength(43)
            });
        }
        catch (error) {
            throw error;
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
        catch (error) {
            throw error;
        }
    });

    test("get minimium pledge amount for a mixnode", async () => {
        try {

            const response = await client.minimumMixnodePledge();

            expect(response.amount).toBe("100000000");
            expect(response.denom).toBe(config.CURRENCY_PREFIX);
        }
        catch (error) {
            throw error;
        }
    });

    test("get minimium gateway pledge amount", async () => {
        try {
            const response = await client.minimumGatewayPledge();

            expect(response.amount).toBe("100000000");
            expect(response.denom).toBe(config.CURRENCY_PREFIX as string);
        }
        catch (error) {
            throw error;
        }
    });

    test("ensure the correct mixnet address is being passed", () => {
        try {
            //should supply the given value from the client init
            const mixnet_contract = client.mixnetContract;
            expect(mixnet_contract).toStrictEqual(config.MIXNET_CONTRACT)
        }
        catch (error) {
            throw error;
        }
    });

    test("ensure the correct vesting address is being passed", () => {
        try {
            const vesting_contract = client.vestingContract;
            expect(vesting_contract).toStrictEqual(config.VESTING_CONTRACT)
        }
        catch (error) {
            throw error;
        }
    });
});
