const currency = require('../../src/currency');

describe("provide unit tests around the the currency module", () => {
    test.skip("convert to native balance", () => {
        const decimal = "12.0346";
        const value = currency.printableBalanceToNative(decimal);
        expect(value).toStrictEqual("12034600");
    });
});

