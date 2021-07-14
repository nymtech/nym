// npx ts-node upload-wasm.ts

import ValidatorClient from "nym-validator-client";
import * as fs from 'fs';

async function newClient(): Promise<ValidatorClient> {
    let contract = "fakeContractAddress"; // we don't have one yet
    let mnemonic = "";
    let admin = ValidatorClient.connect(contract, mnemonic, "http://genesis_validator:26657", "punk");
    return admin;
}

async function main() {
    let admin = await newClient();
    console.log(`admin address: ${admin.address}`);

    // check that we have actually connected to an account, query it to test
    let balance = await admin.getBalance(admin.address);
    console.log(`balance of admin account is: ${balance.amount}${balance.denom}`);

    let wasm = fs.readFileSync("/nym/contracts/mixnet/target/wasm32-unknown-unknown/release/mixnet_contracts.wasm");
    console.log("wasm loaded");

    // dave can upload (note: nobody else can)
    const uploadResult = await admin.upload(admin.address, wasm, undefined, "mixnet contract");//.then((uploadResult) => console.log("Upload from dave succeeded, codeId is: " + uploadResult.codeId)).catch((err) => console.log(err));

    // Instantiate the copy of the option contract
    const { codeId } = uploadResult;
    console.log("code id is", codeId)
    const initMsg = {};
    const options = { memo: "v0.1.0", transferAmount: [{ denom: "upunk", amount: "1000000" }], admin: admin.address }
    let instantiateResult = await admin.instantiate(admin.address, codeId, initMsg, "mixnet contract", options);
    let contractAddress = instantiateResult.contractAddress;
    console.log(`mixnet contract ${contractAddress} instantiated successfully`)
    fs.writeFileSync("current-contract.txt", contractAddress);
}



main();
