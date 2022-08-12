import ContractCache from "../../../../src/endpoints/ContractCache";
import ConfigHandler from "../../../../src/config/configHandler";

let contract: ContractCache;
let config: ConfigHandler;

describe("Get mixnode data", (): void => {
  beforeAll(async (): Promise<void> => {
    contract = new ContractCache();
    config = ConfigHandler.getInstance();
  });

  it("Get all mixnodes", async (): Promise<void> => {
    const response = await contract.getMixnodes();

    console.log(response);

    // response.forEach((x) => {
    //   console.log(x.pledge_amount);
    //   console.log(x.total_delegation);
    //   console.log(x.owner);
    //   console.log(x.layer);
    //   console.log(x.block_height);
    //   console.log(x.mix_node);
    //   console.log(x.proxy);
    //   console.log(x.accumulated_rewards);
    // });
    // expect(typeof response.x).toBe("number");
    // expect(typeof response.saturation).toBe("number");
  });
});
