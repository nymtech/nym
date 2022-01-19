import ValidatorApiQuerier from '../../src/validator-api-querier';
import { config } from '../test-utils/config';

let client: ValidatorApiQuerier;

beforeEach(() => {
    client = new ValidatorApiQuerier(config.VALIDATOR_API as string);
});

describe("init the validator api querier", () => {
    test("get rewarded mixnodes", async () => {
        try {
            //all mixnodes will have their owners address
            let response = await client.getRewardedMixnodes();

            response.forEach((Node) => {
                expect(Node.owner.length).toStrictEqual(43);
            })
        }
        catch (error) {
            throw error;
        }
    });
});

