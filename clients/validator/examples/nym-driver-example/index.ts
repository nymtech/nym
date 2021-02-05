import { SigningCosmWasmClient } from "nym-validator-client"; // maybe change to a NymClient which wraps this guy?
import { connect, loadMnemonic } from "nym-validator-client"; // these could then both go in NymClient. Connect could accept a validator URL.
import * as fs from "fs";

async function main() {
    // get our users set up
    const dave = await buildAccount("dave");
    const fred = await buildAccount("fred");
    const bob = await buildAccount("bob");

    const coins = [{ amount: "5000000000", denom: "unym" }];

    console.log("Sending coins from dave to fred");
    await dave.client.sendTokens(dave.address, fred.address, coins, "Some love for fred!");
    await queryAccount(fred);

    console.log("Sending coins from dave to bob");
    await dave.client.sendTokens(dave.address, bob.address, coins, "Some love for bob!");
    await queryAccount(bob);

    // Upload a new copy of the option contract
    let wasm = fs.readFileSync("../../../../contracts/mixnet/target/wasm32-unknown-unknown/release/mixnet_contracts.wasm");

    // dave can upload (note: nobody else can)
    const uploadResult = await dave.client.upload(dave.address, wasm, undefined, "mixnet contract");
    console.log("Upload from dave succeeded: " + uploadResult.codeId);

    // Instantiate the copy of the option contract
    const { codeId } = uploadResult;
    const initMsg = {};
    const { contractAddress } = await dave.client.instantiate(dave.address, codeId, initMsg, "mixnode contract", { memo: "v0.1.0", transferAmount: [{ denom: "unym", amount: "50000" }] });

    // Use it
    console.log("Now the big moment we've all been waiting for...");
    console.log("Initial topology:");
    await getTopology(contractAddress, dave.client);
    const handle1 = addNode("192.168.1.1", dave, contractAddress).catch(err => {
        console.log(`Error while adding node: ${err}`);
    });
    const handle2 = addNode("192.168.2.1", fred, contractAddress).catch(err => {
        console.log(`Error while adding node: ${err}`);
    });
    const handle3 = addNode("192.168.3.1", bob, contractAddress).catch(err => {
        console.log(`Error while adding node: ${err}`);
    });

    let handles = [handle1, handle2, handle3]
    const combine = Promise.all(handles);
    await combine;

    console.log("Final topology:");
    await getTopology(contractAddress, dave.client);

    console.log("Let's see what is in the topology after we've added the three nodes:");
    let balance = await fred.client.getBalance(contractAddress, "unym");
    console.log(`the mixnet contract currently has: ${balance.amount}${balance.denom}`);

    const before_unbond_balance = await dave.client.getBalance(dave.address, "unym");

    console.log(`Before unbonding, dave's balance is: ${before_unbond_balance.amount}. Now let's try unbonding dave's node`);

    await dave.client.execute(dave.address, contractAddress, { un_register_mixnode: {} });
    console.log("Unbonding succeeded");

    const after_unbond_balance = await dave.client.getBalance(dave.address, "unym");


    console.log(`Dave's account now has: ${after_unbond_balance.amount}`);
    const unbonded_from_dave: number = Number(after_unbond_balance.amount) - Number(before_unbond_balance.amount);
    console.log(`dave's account had ${unbonded_from_dave} restored to it`);

    console.log("has the node been removed from the topology?");
    await getTopology(contractAddress, dave.client);
}

async function addNode(ip: string, account: Account, contractAddress: string) {
    let node = {
        host: ip,
        layer: 1,
        location: "the internet",
        sphinx_key: "mysphinxkey",
        version: "0.9.2",
    };

    const bond = [{ amount: "1000000000", denom: "unym" }];
    await account.client.execute(account.address, contractAddress, { register_mixnode: { mix_node: node } }, "adding mixnode", bond);
    console.log(`account ${account.name} added mixnode with ${ip}`);
}

async function queryAccount(account: Account) {
    const balance = await account.client.getBalance(account.address, "unym");
    console.log(`${account.name} (${account.address}) has: ${balance?.amount}${balance?.denom}`);
}

async function getTopology(contractAddress: string, client: SigningCosmWasmClient) {
    let topology = await client.queryContractSmart(contractAddress, { get_topology: {} });
    console.log(topology.mix_node_bonds);
}

async function buildAccount(name: string): Promise<Account> {
    const mnemonic = loadMnemonic(`../accounts/${name}.key`);
    const { client, address } = await connect(mnemonic, {})
    return new Account(name, client, address);
}

class Account {
    constructor(readonly name: string, readonly client: SigningCosmWasmClient, readonly address: string) { };
}

main();