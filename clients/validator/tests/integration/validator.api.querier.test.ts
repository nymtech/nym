import ValidatorApiQuerier from "../../src/validator-api-querier";
import { config } from "../test-utils/config";

let client: ValidatorApiQuerier;

beforeEach(() => {
  client = new ValidatorApiQuerier(config.VALIDATOR_API);
});

describe("init the validator api querier", () => {
  test.skip("get rewarded mixnodes", async () => {
    try {
      //all mixnodes will have their owners address
      let response = await client.getRewardedMixnodes();

      console.log(response);

      //this is dependany on config and network amend shortly
      response.forEach((Node) => {
        expect(Node.owner.length).toStrictEqual(43);
      });
    } catch (error) {
      throw error;
    }
  });
});
