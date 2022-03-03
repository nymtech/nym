import validator from "../../src/index";

describe("validator build network mnemonic", () => {
  test.skip("get mnemonic", async () => {
    const mnemonic = validator.randomMnemonic();
    const mnemonicCount = mnemonic.split(" ").length;

    expect(mnemonicCount).toStrictEqual(24);
  });
});
