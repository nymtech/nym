// npx ts-node get-topology.ts

import ValidatorClient from "nym-validator-client";

async function createClient(): Promise<ValidatorClient> {
    let contract = "nym18vd8fpwxzck93qlwghaj6arh4p7c5n8974s0uv";
    let dave = "jar travel copy apology neglect disease water fruit gaze possible session normal exclude onion carry matter object dumb tackle assist inspire kind airport crowd";
    let client = ValidatorClient.connect(contract, dave, "http://localhost:26657");
    return client;
}

async function getMixnodes(client: ValidatorClient) {
    await client.refreshMixNodes().then(response => console.log(response)).catch(err => {
        console.log(err);
    });
}


async function main() {
    let client = await createClient();
    getMixnodes(client);
}

main();