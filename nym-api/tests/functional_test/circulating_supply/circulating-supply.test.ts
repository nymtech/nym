import ConfigHandler from "../../src/config/configHandler";
import ContractCache from "../../src/endpoints/CirculatingSupply";

let contract: ContractCache;
let config: ConfigHandler;


describe("Get circulating supply", (): void => {
  beforeAll(async (): Promise<void> => {
    contract = new ContractCache();
    config = ConfigHandler.getInstance();
  });

  it("Get circulating supply amounts", async (): Promise<void> => {
    const response = await contract.getCirculatingSupply();
    
    expect(typeof response.vesting_tokens.amount).toBe('string');

    });
  });