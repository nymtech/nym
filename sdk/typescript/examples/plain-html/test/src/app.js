const puppeteer = require('puppeteer');
const objects = require('./objects');
const environment = require('./environment');
const { expect } = require('chai');


const homepageTest = async () => {
  const browser = await puppeteer.launch();
  const page = await browser.newPage();

  await page.goto(environment.localhost, { waitUntil: 'domcontentloaded' });
  await delay(5000);
  await page.screenshot({ path: 'lol.png' });

  const senderaddress = await page.evaluate(() => {
    return document.querySelector("#sender").value;
  });

  await delay(2000);
  await page.type(objects.messageRecipient, senderaddress);
  // await page.type(objects.id, "bakhakhiahioahgoihoihfoihaoiahfoifhoishoishfoihsoihsfi");
  await page.click(objects.send_button);
  await delay(6000);

  const sentmessage = await page.evaluate(() => {
    return document.querySelector("#output").firstChild.innerText;
  });
  const receivedmesage = await page.evaluate(() => {
    return document.querySelector("#output").lastChild.innerText;
  });

  console.log(receivedmesage);
  console.log(sentmessage);
  expect(receivedmesage).contains("received");
  expect(sentmessage).contains("sent");

  await page.screenshot({ path: 'successful_transaction.png' });
  await browser.close();
};
homepageTest();

function delay(time) {
  return new Promise(function (resolve) {
    setTimeout(resolve, time)
  });
}