import ContractCache from "../../../src/endpoints/ContractCache";

let contract: ContractCache;

describe("Get gateway data", (): void => {
  beforeAll(async (): Promise<void> => {
    contract = new ContractCache();
  });

  it("Get all gateways", async (): Promise<void> => {
    const response = await contract.getGateways();
    response.forEach((gateway) => {
      //overview
      expect(typeof gateway.owner).toBe("string");
      expect(typeof gateway.block_height).toBe("number");

      if (gateway.proxy === null) {
        return true;
      } else {
        expect(typeof gateway.proxy).toBe("string");
      }

      //pledge_amount
      expect(typeof gateway.pledge_amount.denom).toBe("string");
      expect(typeof gateway.pledge_amount.amount).toBe("string");

      //gateway
      expect(typeof gateway.gateway.host).toBe("string");
      expect(typeof gateway.gateway.mix_port).toBe("number");
      expect(typeof gateway.gateway.clients_port).toBe("number");
      expect(typeof gateway.gateway.location).toBe("string");
      expect(typeof gateway.gateway.sphinx_key).toBe("string");
      expect(typeof gateway.gateway.identity_key).toBe("string");
      expect(typeof gateway.gateway.version).toBe("string");
    });
  });

  it("Get blacklisted gateways", async (): Promise<void> => {
    const response = await contract.getBlacklistedGateways();
    if (response === null) {
      // no blacklisted gateways returns an empty array
      expect(response).toBeNull();
    } else {
      response.forEach(function (value) {
        expect(typeof value).toBe("string");
      });
    }
  });
});
