import Page from "./page";
const CountryList = require("../../data/countrylist");

class MixnodeOverview extends Page {
  get menuDropDown(): WebdriverIO.Element {
    return $("#menu");
  }
  get dataKeyTableHeader(): WebdriverIO.Element {
    return $("[data-testid='Owner']");
  }
  get idenitityKeyTableHeader(): WebdriverIO.Element {
    return $("[data-testid='Identity Key']");
  }
  get bondKeyTableHeader(): WebdriverIO.Element {
    return $("[data-testid='Bond']");
  }
  get selfKeyTableHeader(): WebdriverIO.Element {
    return $("[data-testid='Self %']");
  }
  get locationKeyTableHeader(): WebdriverIO.Element {
    return $("[data-testid='Location']");
  }
  get dropDownMenuCount(): WebdriverIO.Element {
    return $("#simple-select");
  }
  get dataGridRowCount(): WebdriverIO.ElementArray {
    return $$(".MuiDataGrid-row");
  }
  get punkAddress(): WebdriverIO.ElementArray {
    return $$("[data-testid='big-dipper-link']");
  }
  get identityKeys(): WebdriverIO.ElementArray {
    return $$("[data-testid='identity-link']");
  }
  get selfPercentage(): WebdriverIO.ElementArray {
    return $$("[data-field='self-percentage']");
  }
  get ipPort(): WebdriverIO.ElementArray {
    return $$("[data-testid='ip-port-address']");
  }
  get location(): WebdriverIO.ElementArray {
    return $$("[data-testid='location']");
  }
  get layer(): WebdriverIO.ElementArray {
    return $$("[data-testid='layer']");
  }

  async clickMenuHeader(element: { click: () => WebdriverIO.Element }) {
    await element.click();
  }

  async rowsDisplayed(number: string, expectedAmount: string) {
    //wait until data grids displayed - clean up
    await browser.pause(1000);

    const dropDownCount = await this.dropDownMenuCount.getText();
    const getRowCount = await this.dataGridRowCount.length;
    expect(getRowCount.toString()).toEqual(expectedAmount);
    expect(number).toEqual(dropDownCount);
  }

  //we can make this generic todo in the future
  async bondAddressesAreValid() {
    const punks = [];

    await this.punkAddress.map(async (elem) => {
      punks.push(await elem.getText());
    });
    for (let i = 0; i < punks.length; i++) {
      //foreach data view, punks should be displayed
      expect(punks[i].length).toEqual(43);
      expect(punks[i].substring(0, 4)).toEqual("punk");
    }
  }

  async identityKeysAreValid() {
    const idKeys = [];
    await this.identityKeys.map(async (elem) => {
      idKeys.push(await elem.getText());
    });
    idKeys.forEach((key) => {
      expect(key.length).toEqual(44);
    });
  }
  async hasValidIpAddresses() {
    const ipAddresses = [];
    await this.ipPort.map(async (elem) => {
      ipAddresses.push(await elem.getText());
    });

    ipAddresses.forEach((ip) => {
      console.log(ip);
      const ipRegex = /^(?!0)(?!.*\.$)((1?\d?\d|25[0-5]|2[0-4]\d)(\.|$)){4}$/;
      if (ipRegex.test(ip)) {
        console.log("----------------");
        console.log(`${ip}} is a valid IP / Host`);
        console.log("----------------");
      } else {
        console.log("bad host address");
        throw `${ip} is bad!`;
      }
    });
  }
  async hasCorrectSelfPercentage() {
    const selfAmount = [];
    await this.selfPercentage.map(async (elem) => {
      selfAmount.push(await elem.getText());
    });

    selfAmount.forEach((amount) => {
      const percentage = amount.substring(0, amount - 1);
      expect(percentage).toBeGreaterThanOrEqual(0);
      expect(percentage).toBeLessThanOrEqual(100);
    });
  }

  async hasValidLocation() {
    const country = new Set();
    await this.location.map(async (elem) => {
      const c = await elem.getText();
      const countryName = c[0].toUpperCase() + c.slice(1).toLowerCase();
      country.add(countryName);
    });

    country.forEach((mixnodeCountry) => {
      expect(mixnodeCountry).toExist(CountryList.countries);
    });
  }

  async hasValidLayer() {
    const layer = [];
    const expectedLayers = [1, 2, 3];
    await this.layer.map(async (elem) => {
      layer.push(await elem.getText());
    });

    layer.forEach((layers) => {
      expect(expectedLayers.includes(Number(layers)));
    });
  }

  open() {
    return super.open("network-components/mixnodes");
  }
}

export default new MixnodeOverview();
