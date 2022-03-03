import { Mock, Times } from "moq.ts";
import { Block, BlockHeader } from "@cosmjs/stargate";
import { CosmWasmClient } from "@cosmjs/cosmwasm-stargate";

describe("implement cosmwasm client test", () => {
  test.only("get height of a block then search for it", async () => {
    let height = Promise.resolve(200);

    let blockHeader = <BlockHeader>{
      version: {
        block: "200",
        app: "testing",
      },
      height: 200,
      chainId: "nym",
      time: "today",
    };

    let block = Promise.resolve(<Block>{
      header: blockHeader,
      id: "test",
      txs: [],
    });

    const getheight = new Mock<CosmWasmClient>()
      .setup((nym) => nym.getHeight())
      .returns(height);

    const getblock = new Mock<CosmWasmClient>()
      .setup((nym) => nym.getBlock(200))
      .returns(block);

    let heightC = getheight.object();
    let blockC = getblock.object();

    let executeHeight = await heightC.getHeight();
    let executeBlock = await blockC.getBlock(200);

    getheight.verify((nym) => nym.getHeight(), Times.Exactly(1));
    getblock.verify((nym) => nym.getBlock(200), Times.Exactly(1));

    expect(executeHeight).toStrictEqual(await height);
    expect(executeBlock.header.height).toStrictEqual(await height);
    expect(executeBlock.header.chainId).toStrictEqual("nym");
  });
});