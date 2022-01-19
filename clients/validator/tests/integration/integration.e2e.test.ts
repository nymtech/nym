import validator from "../../src/index";
import { ExecuteResult } from '@cosmjs/cosmwasm-stargate';
import { Coin } from '@cosmjs/proto-signing';
import { config } from '../test-utils/config';
import { Gateway, GatewayOwnershipResponse, MixNode, MixOwnershipResponse } from '../../src/types';

let response: ExecuteResult;
let validatorClient: validator;
let ownsMixNode: MixOwnershipResponse;
let ownsGateway: GatewayOwnershipResponse;

beforeEach(async () => {
    validatorClient = await validator.connect(
        config.USER_MNEMONIC,
        config.NYMD_URL,
        config.VALIDATOR_API,
        config.CURRENCY_PREFIX,
        config.MIXNET_CONTRACT,
        config.VESTING_CONTRACT
    );
});

describe("long running e2e tests", () => {
    test.only("update mixnode profit percentage", async () => {
        const nodeIdentity = config.MIXNODE_IDENTITY;
        const profitPercent = profitPercentage();

        try {
            //use auto fees - simulated gas
            response = await validatorClient.updateMixnodeConfig(nodeIdentity, 'auto', profitPercent);
        }
        catch (error) {
            throw error;
        }
        try {
            ownsMixNode = await validatorClient.client.ownsMixNode(config.MIXNET_CONTRACT, config.USER_WALLET_ADDRESS);
        }
        catch (error) {
            throw error;
        }
        expect(ownsMixNode.mixnode?.mix_node.profit_margin_percent).toStrictEqual(profitPercent);
    });

    test.only("unbond and bond mixnode", async () => {

        try {
            await validatorClient.unbondMixNode("auto", "unbond-mixnode");
        }
        catch (error) {
            throw error;
        }

        const profitPercent = profitPercentage();

        const mixnodeDetails = <MixNode>{
            host: config.MIXNODE_HOST,
            mix_port: 1789,
            verloc_port: 1790,
            http_api_port: 8080,
            identity_key: config.MIXNODE_IDENTITY,
            sphinx_key: config.MIXNODE_SPHINX_KEY,
            version: config.MIXNODE_VERSION,
            profit_margin_percent: profitPercent
        };

        const pledgeAmount = <Coin>{
            denom: "unymt",
            amount: "100000000"
        };

        try {
            response = await validatorClient.bondMixNode(
                mixnodeDetails,
                config.MIXNODE_SIGNATURE,
                pledgeAmount,
                "auto"
            );
            //see output of events
            //remove shortly
            response.logs.forEach((log) => {
                log.events.forEach((member) => {
                    console.log(member.attributes);
                    console.log(member.type);
                });
            });
        }
        catch (error) {
            throw error;
        }
        
        ownsMixNode = await validatorClient.client.ownsMixNode(config.MIXNET_CONTRACT, config.USER_WALLET_ADDRESS);
        expect(ownsMixNode.mixnode?.mix_node.profit_margin_percent).toStrictEqual(profitPercent);
    });

    test.skip("unbond and bond gateway", async () => {
        //gateway requires different user wallet
        //init client inside test

        try {
            await validatorClient.unbondGateway("auto", "unbonding gateway");
        }
        catch (error) {
            throw error;
        }

        const gateway = <Gateway>{
            host: config.GATEWAY_HOST,
            mix_port: 1789,
            clients_port: 9000,
            version: config.GATEWAY_VERSION,
            sphinx_key: config.GATEWAY_SPHINX,
            identity_key: config.GATEWAY_IDENTITY,
            location: "earth"
        };

        const pledge = pledgeAmount();

        try {
            response = await validatorClient.bondGateway(
                gateway,
                config.GATEWAY_SIGNATURE,
                pledge,
                "auto"
            );
        }
        catch (error) {
            throw error;
        }
        ownsGateway = await validatorClient.client.ownsGateway(config.MIXNET_CONTRACT, config.USER_WALLET_ADDRESS);
        expect(ownsGateway.gateway?.bond_amount).toStrictEqual(pledge.amount);
        expect(ownsGateway.address).toStrictEqual(config.USER_WALLET_ADDRESS);
    });

    test.only("delegate to mixnode, then undelegate", async () => {

        const pledge = pledgeAmount();
        const getBalance = await validatorClient.getBalance(config.USER_WALLET_ADDRESS);
        console.log(getBalance);

        try {
            response = await validatorClient.delegateToMixNode(
                config.MIXNODE_IDENTITY,
                pledge,
                "auto"
            );
            response.logs.forEach((log) => {
                console.log(log.events);
                console.log(log.log);
                console.log(log.msg_index);
            })
        }
        catch (error) {
            throw error;
        }
        try {
            const unbond = await validatorClient.undelegateFromMixNode(
                config.MIXNODE_IDENTITY,
                "auto"
            );

            //see output of events
            //remove shortly
            unbond.logs.forEach((logs) => {
                logs.events.forEach((events) => {
                    console.log(events.type);
                    console.log(events.attributes);
                })
            });
        } catch (error) {
            throw error;
        }
    });
});

const profitPercentage = (): number => {
    return Math.floor(Math.random() * 100) + 1;
}

const pledgeAmount = (): Coin => {
    return <Coin>{
        denom: "unymt",
        amount: "100000000"
    };
}