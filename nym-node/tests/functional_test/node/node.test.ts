import Nodes from "../../src/endpoints/Node";
import { getGatewayIPAddresses } from "../../src/helpers/helper";

describe("Get Node information", (): void => {
  let contract: Nodes;
  let gatewayHosts: string[];
  beforeAll(async (): Promise<void> => {
    try {
      gatewayHosts = await getGatewayIPAddresses();
      // console.log(gatewayHosts);
    } catch (error) {
      throw new Error(`Error fetching gateway IP addresses: ${error.message}`);
    }
  });

  beforeEach(async (): Promise<void> => {
    for (let i = 0; i < gatewayHosts.length; i++) {
      // console.log("currently trying gateway host", gatewayHosts[i]);
      contract = new Nodes(gatewayHosts[i]);
    }
  });

  it("Get build data for the binary running the API", async (): Promise<void> => {
    const response = await contract.getBuildInformation();
    expect(typeof response.binary_name).toBe("string");
    expect(typeof response.build_timestamp).toBe("string");
    expect(typeof response.build_version).toBe("string");
    expect(typeof response.cargo_profile).toBe("string");
    expect(typeof response.commit_branch).toBe("string");
    expect(typeof response.commit_sha).toBe("string");
    expect(typeof response.commit_timestamp).toBe("string");
    expect(typeof response.rustc_channel).toBe("string");
    expect(typeof response.rustc_version).toBe("string");
  });

  it("Get host information for the node", async (): Promise<void> => {
    const response = await contract.getHostInformation();
    response.data.ip_address.forEach((x) => {
      expect(typeof x).toBe("string");
    });
    // expect(typeof response.data.hostname).toBe("string" || "null");
    expect(typeof response.data.hostname === "string" || response.data.hostname === null).toBe(true);
    expect(typeof response.data.keys.ed25519).toBe("string");
    expect(typeof response.data.keys.x25519).toBe("string");
    expect(typeof response.signature).toBe("string");
  });

  it("Get roles supported by the node", async (): Promise<void> => {
    const response = await contract.getSupportedRoles();
    expect(typeof response.gateway_enabled).toBe("boolean");
    expect(typeof response.mixnode_enabled).toBe("boolean");
    expect(typeof response.network_requester_enabled).toBe("boolean");
  });
});
