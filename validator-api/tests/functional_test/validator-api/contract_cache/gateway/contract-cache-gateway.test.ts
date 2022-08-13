import ContractCache from "../../../../src/endpoints/ContractCache";
import ConfigHandler from "../../../../src/config/configHandler";

let contract: ContractCache;
let config: ConfigHandler;


describe("Get gateway data", (): void => {
  beforeAll(async (): Promise<void> => {
    contract = new ContractCache();
    config = ConfigHandler.getInstance();
  });

  it("Get all gateways", async (): Promise<void> => {
    const response = await contract.getGateways();
    response.forEach((gateway) => {
      //overview
      expect(typeof gateway.owner).toStrictEqual('string');
      expect(typeof gateway.block_height).toStrictEqual('number');

      if (typeof gateway.proxy === null) {
        return;
      }
      else {
        expect(typeof gateway.proxy).toStrictEqual('string'); //this is failing as it's returning "object"
      }

      //pledge_amount
      expect(typeof gateway.pledge_amount.denom).toStrictEqual('string');
      expect(typeof gateway.pledge_amount.amount).toStrictEqual('string');

      //gateway
      expect(typeof gateway.gateway.host).toStrictEqual('string');
      expect(typeof gateway.gateway.mix_port).toStrictEqual('number');
      expect(typeof gateway.gateway.clients_port).toStrictEqual('number');
      expect(typeof gateway.gateway.location).toStrictEqual('string');
      expect(typeof gateway.gateway.sphinx_key).toStrictEqual('string');
      expect(typeof gateway.gateway.identity_key).toStrictEqual('string');
      expect(typeof gateway.gateway.version).toStrictEqual('string');
    });
  });

  it("Get blacklisted gateways", async (): Promise<void> => {
    const response = await contract.getBlacklistedGateways();
    response.forEach(function (value) {
      expect(typeof value).toStrictEqual('string');
    });
  });

});
