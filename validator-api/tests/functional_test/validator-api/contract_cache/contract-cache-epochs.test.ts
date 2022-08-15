import ContractCache from "../../../src/endpoints/ContractCache";
import ConfigHandler from "../../../src/config/configHandler";

let contract: ContractCache;
let config: ConfigHandler;


describe("Get epoch info", (): void => {
  beforeAll(async (): Promise<void> => {
    contract = new ContractCache();
    config = ConfigHandler.getInstance();
  });

  it("Get epoch reward params", async (): Promise<void> => {
    const response = await contract.getEpochRewardParams();
    expect(typeof response.epoch_reward_pool).toBe('string');
    expect(typeof response.rewarded_set_size).toBe('string');
    expect(typeof response.active_set_size).toBe('string');
    expect(typeof response.staking_supply).toBe('string');
    expect(typeof response.sybil_resistance_percent).toBe('number');
    expect(typeof response.active_set_work_factor).toBe('number');
  });

  it("Get current epoch", async (): Promise<void> => {
    const response = await contract.getCurrentEpoch();
    expect(typeof response.id).toBe('number');
    expect(typeof response.start).toBe('string');
    expect(typeof response.length.secs).toBe('number');
    expect(typeof response.length.nanos).toBe('number');
  });


});