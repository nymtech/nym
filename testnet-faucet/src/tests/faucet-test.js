const puppeteer = require('puppeteer');
const data = require('./config/test-data.json');
const faucet = require('./pageobjects/elements');
const environments = require('./config/environments');

const WALLET_ADDRESS = data.sendaddress;

const homepageTest = async () => {
  const browser = await puppeteer.launch();
  const page = await browser.newPage();

  await page.goto(environments.localhost);

  await page.waitForSelector(faucet.inputAddress);
  await page.type(faucet.inputAddress, WALLET_ADDRESS);
  await page.type(faucet.inputAmount, '5');
  await page.click(faucet.requestTokens);

  //wait the longest period of time for the transaction to finish
  await page.waitForTimeout(8000);

  await page.waitForSelector('.balance:not(:empty)');

  const element = await page.$(faucet.transaction);
  const text = await page.evaluate((element) => element.textContent, element);

  //Successfully transfered 5 upunk to address `punkaddress`
  //input test framework to assert expected response
  console.log(text);

  await page.screenshot({ path: 'successful_transaction.png' });

  await browser.close();
};
homepageTest();
