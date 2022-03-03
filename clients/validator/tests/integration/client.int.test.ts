import validatorClient from "../../src/index";
import { config } from "../test-utils/config";

let client: validatorClient;
let mnemonic: string;

beforeEach(async () => {
  mnemonic = validatorClient.randomMnemonic();
  client = await validatorClient.connect(
    mnemonic,
    config.NYMD_URL,
    config.VALIDATOR_API,
    config.CURRENCY_DENOM,
    config.MIXNET_CONTRACT,
    config.VESTING_CONTRACT
  );
});

describe("perform a few non expensive network calls with the validator client", () => {
  test.skip("get all cached mixnodes", async () => {
    try {
      const response = await client.getCachedMixnodes();

      //expect all mixnodes to have their owner address
      response.forEach((mixnodeDetails) => {
        expect(mixnodeDetails.owner).toHaveLength(43);
      });
    } catch (error) {
      throw error;
    }
  });

  test.skip("get minimium pledge amount for a mixnode", async () => {
    try {
      const response = await client.minimumMixnodePledge();

      expect(response.amount).toBe("100000000");
      expect(response.denom).toBe(config.CURRENCY_DENOM);
    } catch (error) {
      throw error;
    }
  });

  test.skip("get minimium gateway pledge amount", async () => {
    try {
      const response = await client.minimumGatewayPledge();

      expect(response.amount).toBe("100000000");
      expect(response.denom).toBe(config.CURRENCY_DENOM as string);
    } catch (error) {
      throw error;
    }
  });

  test.skip("ensure the correct mixnet address is being passed", () => {
    try {
      //should supply the given value from the client init
      const mixnet_contract = client.mixnetContract;
      expect(mixnet_contract).toStrictEqual(config.MIXNET_CONTRACT);
    } catch (error) {
      throw error;
    }
  });

  test.skip("ensure the correct vesting address is being passed", () => {
    try {
      const vesting_contract = client.vestingContract;
      expect(vesting_contract).toStrictEqual(config.VESTING_CONTRACT);
    } catch (error) {
      throw error;
    }
  });
});