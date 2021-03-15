import ValidatorClient from "nym-validator-client";
import fs from "fs";

// npx ts-node upload-wasm.ts

async function newClient(mnemonic: string): Promise<ValidatorClient> {
    let contract = "fakeContractAddress"; // we don't have one yet
    let client = ValidatorClient.connect(contract, mnemonic, "http://localhost:26657");
    return client;
}

async function main() {
    let adminPass = "jar travel copy apology neglect disease water fruit gaze possible session normal exclude onion carry matter object dumb tackle assist inspire kind airport crowd";
    let admin = await newClient(adminPass);
    console.log("dave address: ", admin.address);

    let wasm = fs.readFileSync("../../../../contracts/mixnet/target/wasm32-unknown-unknown/release/mixnet_contracts.wasm");
    console.log("wasm loaded");

    // dave can upload (note: nobody else can)
    const uploadResult = await admin.upload(admin.address, wasm, undefined, "mixnet contract");
    console.log("Upload from dave succeeded, codeId is: " + uploadResult.codeId);

    // Instantiate the copy of the option contract
    const { codeId } = uploadResult;
    const initMsg = {};
    let instantiateResult = await admin.instantiate(admin.address, codeId, initMsg, "mixnet contract", { memo: "v0.1.0", transferAmount: [{ denom: "unym", amount: "50000" }] });
    let contractAddress = instantiateResult.contractAddress;
    console.log(`mixnet contract ${contractAddress} instantiated successfully`)
    fs.writeFileSync("current-contract.txt", contractAddress);
}

async function buildKeyPath(name: string, testAccountsDir: string): Promise<string> {
    return `${testAccountsDir}${name}.key`;
}

main();





