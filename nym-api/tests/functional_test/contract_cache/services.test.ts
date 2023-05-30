import ContractCache from "../../src/endpoints/ContractCache";
import ConfigHandler from "../../src/config/configHandler";

let contract: ContractCache;
let config: ConfigHandler;

describe("Get service provider info", (): void => {
    beforeAll(async (): Promise<void> => {
        contract = new ContractCache();
        config = ConfigHandler.getInstance();
    });

    it("Get service providers", async (): Promise<void> => {
        const response = await contract.getServiceProviders();
        if ("[service_id]" in response) {
            response.services.forEach((x) => {
                expect(typeof x.service.nym_address.address).toBe("string");
                expect(typeof x.service.service_type).toBe("string");
                expect(typeof x.service.block_height).toBe("number");
                expect(typeof x.service.announcer).toBe("string");
                expect(typeof x.service.deposit.amount).toBe("string");
                expect(typeof x.service.deposit.denom).toBe("string");
            });
        } else if ("[ ]" in response) {
            return;
        }
    });
});