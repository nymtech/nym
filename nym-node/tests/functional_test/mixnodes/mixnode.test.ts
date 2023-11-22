import Mixnode from "../../src/endpoints/Mixnodes";
import { getMixnodeIPAddresses } from '../../src/helpers/helper';

describe("Get mixnode related info", (): void => {
    let contract: Mixnode;
    let mixnodeHosts: string[];
    beforeAll(async (): Promise<void> => {
        try {
            mixnodeHosts = await getMixnodeIPAddresses();
            console.log(mixnodeHosts)
        } catch (error) {
            throw new Error(`Error fetching mixnode IP addresses: ${error.message}`);
        }
    });

    beforeEach(async (): Promise<void> => {
        // Testing only 3 nodes from the list
        for (let i = 3; i < mixnodeHosts.length; i++) {
            console.log("currently trying mixnode host", mixnodeHosts[i])
            contract = new Mixnode(mixnodeHosts[i]);
        }
    });

    it("Get mixnode details", async (): Promise<void> => {
        const response = await contract.getMixnodeInfo();
        // This response is currently just empty {}, amend it when it changes
        expect(response).toEqual({});
    });
});
