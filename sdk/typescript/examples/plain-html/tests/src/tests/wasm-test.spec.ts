import { expect } from 'chai';
const selectors = require('../selectors/attr.json');
import kermit from '../utils/kermit';
const config = require('../config/config.json');
import delay from '../utils/helper';

describe('run base test', async () => {
  let browser;
  let page;

  before(async () => {
    browser = await kermit();
    page = await browser.newPage();
    await page.goto(config.localhost);
    await delay(5000);
  });

  after(async () => {
    await page.screenshot({ path: 'transaction.png' });
    await browser.close();
  });

  it('Validate an address can be pasted', async () => {
    const senderaddress = await page.evaluate(() => {
      return (document.querySelector('#sender') as HTMLInputElement).value;
    });
    await page.type(selectors.recipient, senderaddress);
  });

  it('Validate a mesage can be typed', async () => {
    await page.click(selectors.id, { clickCount: 3 });
    await page.type(selectors.id, "Hi, I'm a test");
  });

  it('Validate the message can be sent and received', async () => {
    await page.click(selectors.send_button);
    await delay(1500);

    const sentmessage = await page.evaluate(() => {
      return (document.querySelector('#output') as Element).firstChild.textContent;
    });
    const receivedmesage = await page.evaluate(() => {
      return (document.querySelector('#output') as Element).lastChild.textContent;
    });

    console.log(receivedmesage);
    console.log(sentmessage);
    expect(receivedmesage).contains('received');
    expect(sentmessage).contains('sent');
  });
});
