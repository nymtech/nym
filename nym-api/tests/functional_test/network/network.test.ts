import NetworkTypes from "../../src/endpoints/Network";
let contract: NetworkTypes;

describe("Get network and contract details", (): void => {
    beforeAll(async (): Promise<void> => {
        contract = new NetworkTypes();
    });

    it("Get network details", async (): Promise<void> => {
        const response = await contract.getNetworkDetails();
        expect(typeof response.network.network_name).toBe("string");
    });

    it("Get nym contract info", async (): Promise<void> => {
        const response = await contract.getNymContractInfo();
        for (const key in response) {
            if (response.hasOwnProperty(key)) {
                const additionalProp = response[key];
                expect(typeof additionalProp.address).toBe("string");
                if ("build_timestamp" in response) {
                    expect(typeof additionalProp.details.contract).toBe("string");
                    expect(typeof additionalProp.details.version).toBe("string");
                }
                else if (additionalProp.details === null) {
                    expect(additionalProp.details).toBeNull();
                }
            }
        }
    });

    it("Get nym contract info detailed", async (): Promise<void> => {
        const response = await contract.getNymContractDetailedInfo();
        for (const key in response) {
            if (response.hasOwnProperty(key)) {
                const additionalProp = response[key];
                expect(typeof additionalProp.address).toBe("string");
                if ("build_timestamp" in response) {
                    expect(typeof additionalProp.details.build_timestamp).toBe("string");
                    expect(typeof additionalProp.details.rustc_version).toBe("string");
                }
                else if (additionalProp.details === null) {
                    expect(additionalProp.details).toBeNull();
                }
            }
        }
    });
});
