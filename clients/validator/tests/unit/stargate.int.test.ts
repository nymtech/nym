const stargate = require("../../src/stargate-helper");
import { config } from "../test-utils/config";

describe("test the stargate functions within the project", () => {
  test.skip("gas price is returned correctly", () => {
    const nymCurrency = config.CURRENCY_DENOM;
    const getGasPrice = stargate.nymGasPrice(nymCurrency);

    expect(getGasPrice.denom).toBe(`u${nymCurrency}`);
  });

  test.skip("provide invalid type returns an error message", () => {
    //pass invalid type
    expect(() => {
      const nymCurrency = 13;
      stargate.nymGasPrice(nymCurrency);
    }).toThrow("13 is not of type string");
  });
});
