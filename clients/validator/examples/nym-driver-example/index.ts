import { SigningCosmWasmClient } from "nym-validator-client"; // maybe change to a NymClient which wraps this guy?
import { connect, loadMnemonic, randomMnemonic, mnemonicToAddress } from "nym-validator-client"; // these could then both go in NymClient. Connect could accept a validator URL.
import * as fs from "fs";

async function main() {
    // get our users set up
    const dave = await buildAccount("dave");
    const fred = await buildAccount("fred");
    const bob = await buildAccount("bob");
    const thief = await buildAccount("thief");


    const coins2000_nym = [{ amount: "2000000000", denom: "unym" }];

    // console.log("Sending coins from dave to fred");
    // await dave.client.sendTokens(dave.address, fred.address, coins2000_nym, "Some love for fred!");

    // console.log("Sending coins from dave to bob");
    // await dave.client.sendTokens(dave.address, bob.address, coins2000_nym, "Some love for bob!");

    // console.log("Sending coins from dave to thief");
    // await dave.client.sendTokens(dave.address, thief.address, coins2000_nym, "Some love for thief!");

    // await queryAccount(fred);
    // await queryAccount(bob);
    // await queryAccount(thief);

    // Upload a new copy of the option contract
    // let wasm = fs.readFileSync("../../../../contracts/mixnet/target/wasm32-unknown-unknown/release/mixnet_contracts.wasm");

    // dave can upload (note: nobody else can)
    // const uploadResult = await dave.client.upload(dave.address, wasm, undefined, "mixnet contract");
    // console.log("Upload from dave succeeded, codeId is: " + uploadResult.codeId);

    // Instantiate the copy of the option contract
    // const { codeId } = uploadResult;
    // const initMsg = {};
    // const { contractAddress } = await dave.client.instantiate(dave.address, codeId, initMsg, "mixnet contract", { memo: "v0.1.0", transferAmount: [{ denom: "unym", amount: "50000" }] });
    // console.log(`mixnet contract ${contractAddress} instantiated successfully`)

    const contractAddress = "nym10pyejy66429refv3g35g2t7am0was7ya69su6d";

    // Use it
    console.log("Now the big moment we've all been waiting for...");
    console.log("Initial topology:");
    await getTopology(contractAddress, dave.client);

    console.log("Adding nodes from dave, bob, and fred...");


    for (var i = 1; i < 3000; i++) {
        let mnemonic = await randomMnemonic();
        let account = await newAccount(mnemonic);

        await dave.client.sendTokens(dave.address, account.address, coins2000_nym, `Token send to random address`);
        await addNode("192.168.1.1", account, contractAddress).catch(err => {
            console.log(`Error while adding node: ${err}`);
        });
    }


    console.log("Let's see what is in the topology after we've added the three nodes:");
    await getTopology(contractAddress, dave.client);

    // let balance = await fred.client.getBalance(contractAddress, "unym");
    // console.log(`the mixnet contract currently has: ${balance.amount}${balance.denom}`);

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

    console.log("trying to unbond a node with an account that doesn't own a mixnode bond");
    await thief.client.execute(thief.address, contractAddress, { un_register_mixnode: {} }).catch(err => {
        console.log(`Error unbonding node: ${err}`)
    });
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
    console.log(`account ${account.address} added mixnode with ${ip}`);
}

async function queryAccount(account: Account) {
    const balance = await account.client.getBalance(account.address, "unym");
    console.log(`${account.name} (${account.address}) has: ${balance?.amount}${balance?.denom}`);
}

async function getTopology(contractAddress: string, client: SigningCosmWasmClient) {// : Promise<Topology> {
    let topology = await client.queryContractSmart(contractAddress, { get_topology: {} });
    console.log(topology.mix_node_bonds);
    console.log(`length is: ${topology.mix_node_bonds.length}.`);
}

async function buildAccount(name: string): Promise<Account> {
    const mnemonic = loadMnemonic(`../accounts/${name}.key`);
    const { client, address } = await connect(mnemonic, {})
    return new Account(name, client, address);
}

async function newAccount(mnemonic: string): Promise<Account> {
    const { client, address } = await connect(mnemonic, {});
    return new Account(address, client, address);

}

class Account {
    constructor(readonly name: string, readonly client: SigningCosmWasmClient, readonly address: string) { };
}

class Topology {
    constructor(readonly mixNodes: [], readonly validators: []) { };
}

main();