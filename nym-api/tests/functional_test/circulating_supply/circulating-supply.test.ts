import ContractCache from "../../src/endpoints/CirculatingSupply";
let contract: ContractCache;

describe("Get circulating supply", (): void => {
  beforeAll(async (): Promise<void> => {
    contract = new ContractCache();
  });

  it("Get circulating supply amounts", async (): Promise<void> => {
    const response = await contract.getCirculatingSupply();
    const totalsupply: number = +response.total_supply.amount;
    const mixmining: number = +response.mixmining_reserve.amount;
    const vest: number = +response.vesting_tokens.amount;
    const circsupply: number = +response.circulating_supply.amount;

    expect(typeof response.vesting_tokens.amount).toBe("string");
    expect(totalsupply - mixmining - vest).toStrictEqual(circsupply);
  });

  it("Get total supply value", async (): Promise<void> => {
    const response = await contract.getTotalSupplyValue();
    expect(response).toStrictEqual(1000000000);
  });

  it("Get circulating supply value", async (): Promise<void> => {
    const response = await contract.getCirculatingSupplyValue();
    expect(typeof response).toBe("number");
  });
});
