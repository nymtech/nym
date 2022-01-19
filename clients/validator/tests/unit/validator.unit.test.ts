import validator from "../../src/index";
import { config } from '../test-utils/config';

const NETWORK_DENOM = config.CURRENCY_PREFIX;

describe("perform basic interactions with the validator", () => {
    test("build client and get all mixnodes", async () => {
        const mnemonic = validator.randomMnemonic();
        const mnemonicCount = mnemonic.split(" ").length;
       
        expect(mnemonicCount).toStrictEqual(24);
    });

    test("build client and get all mixnodes", async () => {
        const buildMnemonic = validator.randomMnemonic();
        const mnemonic = await validator.mnemonicToAddress(buildMnemonic, NETWORK_DENOM);
        
        expect(mnemonic).toHaveLength(43);
        expect(mnemonic).toContain(NETWORK_DENOM);
    });
});
