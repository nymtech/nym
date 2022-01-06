const stargate = require("../../src/stargate-helper");
import { config } from '../test-utils/config';

describe("test the stargate functions within the client", () => {
    test("test that the gas price is returned correctly", () => {
        const nymCurrency = config.CURRENCY_PREFIX as string;
        const getGasPrice = stargate.nymGasPrice(nymCurrency);
        expect(getGasPrice.denom).toBe(`u${nymCurrency}`);
    });

    test("provide invalid type returns an error message", () => {
        //pass invalid type
        expect(() => {
            const nymCurrency = 13;
            stargate.nymGasPrice(nymCurrency);
        }).toThrow("13 is not of type string");
    });

    //provide test for downloading wasm 
    //mock this test  
    // test.skip("providing nothing returns", async () => {
    //     //pass invalid type
    //     const downloadWasm = stargate.downloadWasm("http://localhost");
    //     console.log(downloadWasm);
    // })
});

