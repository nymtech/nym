import mixnodeOverviewPage from "../pageobjects/mixnode.overview";
import helper from "../../helpers/actionhelper";

const config = require("../../wdio.conf").config;
const mixnodeUrl = `${config.baseUrl}/network-components/mixnodes`;

describe("Access the mixnode overview page", () => {
  it("should validate all the data grids and it's contents", async () => {
    await mixnodeOverviewPage.open();

    await helper.waitUntilPageLoads();

    await helper.waitUntilCickable(mixnodeOverviewPage.dropDownMenuCount);

    await helper.waitUntilPageLoads();
    //default display view renders 10 rows
    await mixnodeOverviewPage.rowsDisplayed("10", "10");

    expect(await browser.getUrl()).toEqual(mixnodeUrl);

    await mixnodeOverviewPage.bondAddressesAreValid();

    await mixnodeOverviewPage.identityKeysAreValid();

    await mixnodeOverviewPage.hasValidIpAddresses();

    await mixnodeOverviewPage.hasCorrectSelfPercentage();

    await mixnodeOverviewPage.hasValidLocation();

    await mixnodeOverviewPage.hasValidLayer();
  });
});
