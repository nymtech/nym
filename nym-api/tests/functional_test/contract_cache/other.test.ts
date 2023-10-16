import ContractCache from "../../src/endpoints/ContractCache";
let contract: ContractCache;

describe("Get service provider info", (): void => {
  beforeAll(async (): Promise<void> => {
    contract = new ContractCache();
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

  it("Get Nym address names", async (): Promise<void> => {
    const response = await contract.getNymAddressNames();
    if ("[id]" in response) {
      response.names.forEach((x) => {
        expect(typeof x.name.name).toBe("string");
        expect(typeof x.name.address.gateway_id).toBe("string");
        expect(typeof x.id).toBe("number");
        
      });
    } else if ("[ ]" in response) {
      return;
    }
  });
});
