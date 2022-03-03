import { Mock, Times } from "moq.ts";
import { INymdQuery } from "../../src/query-client";

describe("nym-client mocks", () => {
  test.only("gets interval rewarding percent", async () => {
    let contract = "mixnet_contract";
    let response = Promise.resolve(Number(2));

    const client = new Mock<INymdQuery>()
      .setup((nym) => nym.getIntervalRewardPercent(contract))
      .returns(response);

    const obj = client.object();

    let execute = await obj.getIntervalRewardPercent(contract);

    client.verify(
      (nym) => nym.getIntervalRewardPercent(contract),
      Times.Exactly(1)
    );

    expect(execute).toStrictEqual(await response);
  });
});

