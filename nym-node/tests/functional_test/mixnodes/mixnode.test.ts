// import Mixnode from "../../src/endpoints/Mixnodes";
// import { getGatewayIPAddresses } from '../../src/helpers/helper';

// describe("Get mixnode related info", (): void => {
//     let contract: Mixnode;
//     let gatewayHosts: string[];
//     beforeAll(async (): Promise<void> => {
//         try {
//             gatewayHosts = await getGatewayIPAddresses();
//             console.log(gatewayHosts)
//         } catch (error) {
//             throw new Error(`Error fetching gateway IP addresses: ${error.message}`);
//         }
//     });

//     beforeEach(async (): Promise<void> => {
//         for (let i = 0; i < gatewayHosts.length; i++) {
//             console.log("currently trying gateway host", gatewayHosts[i])
//             contract = new Mixnode(gatewayHosts[i]);
//         }
//     });

//     it("Get mixnode details", async (): Promise<void> => {
//         const response = await contract.getMixnodeInfo();
//         // TODO implement checks here
//     });

// });
