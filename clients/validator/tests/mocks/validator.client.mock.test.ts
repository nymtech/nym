import { Coin } from "@cosmjs/proto-signing";
import { Mock, Times } from "moq.ts";
import ValidatorClient from "../../src/index";
import { DeliverTxResponse, logs } from "@cosmjs/stargate";
import { Gateway, MixNode } from "../../src/types";
import { ExecuteResult } from "@cosmjs/cosmwasm-stargate";
import { config } from "../test-utils/config";
import { buildWallet, buildCoin, profitPercentage } from "../test-utils/utils";
import { promiseExecuteResult } from "../test-utils/expectedResults";

describe("mock validator client tests", () => {
  test.skip("token transfer", async () => {

    //arrange
    //todo -- add more here
    let recipientAddress = "nymt14ev4p8qaa7ayr06cg3z7y2u2kxc9a8f4h9gkch";
    let sender = "nymt1cv59jumgvz2chn7ffst8tzvnapqzp282m5vat2";

    const coin = buildCoin("50000", "nymt");

    let transaction = Promise.resolve(<DeliverTxResponse>{
      code: 0,
      height: 1208302,
      rawLog: "[]",
      transactionHash:
        "9C7BF465AB5CAB0D62446CBB251CF89CD173A640C5DE8DBC14A4BB950916114E",
      gasUsed: 65042,
      gasWanted: 67977,
    });

    console.log(transaction);

    let mockClient = new Mock<ValidatorClient>()
      .setup((nym) => nym.send(recipientAddress, [coin], "auto", "test")).returns(transaction);

    let token = mockClient.object();

    //act 
    let response = await token.send(recipientAddress, [coin], "auto", "test");
    
    //assert
    mockClient.verify(cl => cl.send(recipientAddress, [coin], "auto"), Times.Exactly(1));
  });

  test.only("bond mixnode test", async () => {
    //arrange
    let ownerSignature = "ownersignature";
    let coin = buildCoin("50000", "nymt");
    let expectedResult = promiseExecuteResult();

    const profitPercent = profitPercentage();

    const mixnode = <MixNode>{
      host: "1.1.1.1",
      mix_port: 1789,
      verloc_port: 1790,
      http_api_port: 8080,
      identity_key: "identity",
      sphinx_key: "identity",
      version: "0.12.1",
      profit_margin_percent: profitPercent,
    };

    let client = new Mock<ValidatorClient>()
      .setup((client) =>
        client.bondMixNode(mixnode, ownerSignature, coin, "auto")
      )
      .returns(expectedResult);

    let mixnodeBond = client.object();

    //act
    let response = await mixnodeBond.bondMixNode(
      mixnode,
      ownerSignature,
      coin,
      "auto"
    );
    client.verify((cl) =>
      cl.bondMixNode(mixnode, ownerSignature, coin, "auto")
    );

    //assert
    expect(response.logs[0].log).toStrictEqual("test");
    expect(response.transactionHash).toStrictEqual(
      "9C7BF465AB5CAB0D62446CBB251CF89CD173A640C5DE8DBC14A4BB950916114E"
    );
  });

  test.only("un-bond mixnode", async () => {
    //arrange
    let expectedResult = promiseExecuteResult();

    let client = new Mock<ValidatorClient>()
      .setup((client) => client.unbondMixNode("auto"))
      .returns(expectedResult);

    let unbondNode = client.object();

    //act
    let response = await unbondNode.unbondMixNode("auto");
    client.verify((cl) => cl.unbondMixNode("auto"));

    //assert
    expect(response.logs[0].log).toStrictEqual("test");
    expect(response.transactionHash).toStrictEqual(
      "9C7BF465AB5CAB0D62446CBB251CF89CD173A640C5DE8DBC14A4BB950916114E"
    );
  });


  test.only("bond gateway", async () => {
    //arrange
    let expectedResult = promiseExecuteResult();
    let ownerSignature = "ownersigntature";
    let coin = buildCoin("50000", "nymt");

    const gateway = <Gateway>{
      host: '1.2.3.4',
      mix_port: 1789,
      clients_port: 9000,
      version: "0.12.1",
      sphinx_key: "sphinx_key",
      identity_key: "identity_key",
      location: "earth"
  };

    let client = new Mock<ValidatorClient>()
      .setup((client) => client.bondGateway(gateway, ownerSignature, coin, "auto", "memo"))
      .returns(expectedResult);

    let mock = client.object();

    //act
    let response = await mock.bondGateway(gateway, ownerSignature, coin, "auto", "memo");
    client.verify((cl) => cl.bondGateway(gateway, ownerSignature, coin, "auto", "memo"));

    //assert
    expect(response.logs[0].log).toStrictEqual("test");
    expect(response.transactionHash).toStrictEqual(
      "9C7BF465AB5CAB0D62446CBB251CF89CD173A640C5DE8DBC14A4BB950916114E"
    );
  });

  test.only("unbond gateway", async () => {
    //arrange
    let expectedResult = promiseExecuteResult();
    let client = new Mock<ValidatorClient>()
      .setup((client) => client.unbondGateway())
      .returns(expectedResult);

    let mock = client.object();

    //act
    let response = await mock.unbondGateway();
    client.verify((cl) => cl.unbondGateway());

    //assert
    expect(response.logs[0].log).toStrictEqual("test");
    expect(response.transactionHash).toStrictEqual(
      "9C7BF465AB5CAB0D62446CBB251CF89CD173A640C5DE8DBC14A4BB950916114E"
    );
  });

  test.only("retrieve a newly created account and the balance should be empty", async () => {
    let nymWallet = await buildWallet();

    let coin = Promise.resolve(<Coin>{
      denom: `${config.CURRENCY_DENOM}`,
      amount: "0",
    });

    let client = new Mock<ValidatorClient>()
      .setup((nym) => nym.getBalance(nymWallet))
      .returns(coin);

    let obj = client.object();

    let execute = await obj.getBalance(nymWallet);

    client.verify((nym) => nym.getBalance(nymWallet), Times.Exactly(1));

    expect(execute).toStrictEqual(await coin);
  });
});


