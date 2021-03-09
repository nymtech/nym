// npx ts-node get-balance.ts

import ValidatorClient from "nym-validator-client";

async function newClient(mnemonic: string): Promise<ValidatorClient> {
    let contract = "nym18vd8fpwxzck93qlwghaj6arh4p7c5n8974s0uv";
    let client = ValidatorClient.connect(contract, mnemonic, "http://localhost:26657");
    return client;
}

async function main() {
    let davePass = "jar travel copy apology neglect disease water fruit gaze possible session normal exclude onion carry matter object dumb tackle assist inspire kind airport crowd";
    let dave = await newClient(davePass);
    console.log("dave address: ", dave.address);
    await dave.getBalance(dave.address)
        .then(response => console.log(`dave: ${JSON.stringify(response)}`))
        .catch(err => console.log(err));

    let fredPass = "pride moral airport someone involve rabbit else napkin cheese hello tent stove rabbit mean help small ship embark concert aim journey void fly output";
    let fred = await newClient(fredPass);
    console.log("fred address: ", fred.address);
    await fred.getBalance(fred.address)
        .then(response => console.log(`fred: ${JSON.stringify(response)}`))
        .catch(err => console.log(err));
}

main();