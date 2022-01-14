import SigningClient from '../../src/signing-client';
import validator from "../../src/index";
import { ExecuteResult } from '@cosmjs/cosmwasm-stargate';
import { Coin, DirectSecp256k1HdWallet } from '@cosmjs/proto-signing';
import { config } from '../test-utils/config';
import { Gateway, MixNode } from '../../src/types';

let signingClient: SigningClient;
let mnemonic: string;
let response: ExecuteResult;
let wallet: DirectSecp256k1HdWallet;

beforeEach(async () => {
    if (config.USER_MNEMONIC  != undefined) {
        mnemonic = config.USER_MNEMONIC as string;
    }
    else {
        mnemonic = validator.randomMnemonic();
    }
   
    wallet = await DirectSecp256k1HdWallet.fromMnemonic(mnemonic);
    
    signingClient = await SigningClient.connectWithNymSigner(
        wallet,
        config.NYMD_URL as string,
        config.VALIDATOR_API as string,
        config.CURRENCY_PREFIX as string);
});

describe("simple actions to simulate a users actions for nym", () => {
    test.only("bond mixnode", async () => {

        //provide your mixnode details
        // - ownersignature
        // - nym wallet address
        // - Mixnode model
        const nymwalletaddress = "string";

        const mixnodeDetails = <MixNode>{
            host: "1.1.1.1",
            mix_port: 1789,
            verloc_port: 1790,
            http_api_port: 8080,
            identity_key: "",
            sphinx_key: "",
            version: "0.12.1"
        }

        const ownerSignature = "some signature";

        const coin = <Coin>{
            denom: "unymt",
            amount: "1000"
        };

        try {
            response = await signingClient.bondMixNode(
                nymwalletaddress,
                mixnodeDetails,
                ownerSignature,
                coin
            );
            //todo - do something
            //example = expect(response.logs[0].events[0].type).toStrictEqual("test");
        }
        catch (e) {
            console.log(e);
        }
    });
    test.only("bond gateway", async () => {

        //provide your mixnode details
        // - ownersignature
        // - nym wallet address
        // - Gateway model
        // - the minimum pledge amount to the gateway

        const nymwalletaddress = "string";

        const gateway = <Gateway>{
            host: "1.1.1.1",
            mix_port: 1789,
            clients_port: 9000,
            version: "0.12.1",
            sphinx_key: "",
            identity_key: "",
            location: "earth"
        };
       
        const ownerSignature = "some signature";
       
        const coin = <Coin>{
            denom: "unymt",
            amount: "10000"
        };

        try {
            response = await signingClient.bondGateway(
                nymwalletaddress,
                gateway,
                ownerSignature,
                coin
            );
            //todo - do something
            //example = expect(response.logs[0].events[0].type).toStrictEqual("test");
        }
        catch (e) {
            console.log(e);
        }
    });
});