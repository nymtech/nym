import overviewPage from "../pageobjects/overview.page";
import actionhelper from "../../helpers/actionhelper";

const urls = require("../../helpers/urls");
const config = require("../../wdio.conf").config;

const mixnodeUrl = `${config.baseUrl}/network-components/mixnodes`;
const gatewayUrl = `${config.baseUrl}/network-components/gateways`;

describe("Access the overview of the nym explorer", () => {
  it("should match the url from the base configuration", async () => {
    await overviewPage.open();

    const getUrl = await browser.getUrl();

    expect(getUrl).toEqual(`${config.baseUrl}/overview`);
  });

  it("selecting mixnodes opens the mixnode page", async () => {
    await overviewPage.selectMixnode();

    const getUrl = await browser.getUrl();

    expect(getUrl).toEqual(mixnodeUrl);

    await browser.back();
  });

  it("selecting gateways opens the gateways page", async () => {
    await overviewPage.selectGateways();

    const getUrl = await browser.getUrl();

    expect(getUrl).toEqual(gatewayUrl);

    await browser.back();
  });

  it("selecting validators opens the block explorer", async () => {
    //by selecting validators it opens up the block explorer
    await actionhelper.openSwitchToNewTab(overviewPage.validatorLink);

    await actionhelper.waitUntilPageLoads();

    const getUrl = await browser.getUrl();

    expect(getUrl).toEqual(urls.blockexplorer);

    await browser.closeWindow();
  });
});
