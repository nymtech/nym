import ValidatorApiQuerier from '../../src/validator-api-querier';
import { config } from '../test-utils/config';
import { now, elapsed} from '../test-utils/utils';

let client: ValidatorApiQuerier;

beforeEach(() => {
    client = new ValidatorApiQuerier(config.VALIDATOR_API as string);
});

//todos
//we want to mock the majority of these tests
//and keep a few integration tests in place

describe("call out to the validator api and run queries", () => {
    test.skip("build client and get all mixnodes", async () => {
        try {
            //this test was currently ran against a set of prefix data
            //this will change
            const ownerAddress = "nymt1ydqkmz0ddpvkd3l0vyf8k5xjrqtcnxxvhlpdsr";
            let response = await client.getActiveMixnodes();
            expect(response[0].owner).toStrictEqual(ownerAddress);
        }
        catch (e) {
            console.log(e);
        }
    });

    test("get rewarded mixnodes", async () => {
        try {
            // we assume that all mixnodes will have their owners address
            // also active sets will determine rewarded mixnodes
            let response = await client.getRewardedMixnodes();

            response.forEach(rNode => {
                expect(rNode.owner.length).toStrictEqual(43);
            })
        }
        catch (e) {
            console.log(e);
        }
    });

    test("get cached gateways and it should be six", async () => {
        try {
            //current gateways running in sandbox-testnet 6
            let response = await client.getCachedGateways();
            expect(response.length).toStrictEqual(6);
        }
        catch (e) {
            console.log(e);
        }
    });


    test("get cached mixnodes", async () => {
        try {
            const start = now();
            let response = await client.getCachedMixnodes();
            response.forEach(mixnode => {
                expect(mixnode.owner.length).toStrictEqual(43);
            })
            console.log(elapsed(start, true));
        }
        catch (e) {
            console.log(e);
        }
    });
});

