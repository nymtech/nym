/* eslint-disable no-await-in-loop */
import puppeteer from 'puppeteer';

const TIMEOUT = 30_000;

function sleep(time) {
  return new Promise((resolve) => {
    setTimeout(resolve, time);
  });
}

export async function runTests(log) {
  console.log('🟡 Starting puppeteer');
  // launch Chrome and navigate
  const browser = await puppeteer.launch({ headless: 'new' });
  const page = await browser.newPage();
  await page.setDefaultNavigationTimeout(TIMEOUT); // timeout 60 seconds now

  const errors = [];

  page.on('console', (message) => {
    let isError = false;
    if (message.type() === 'error') {
      if (!message.location()?.url?.endsWith('favicon.ico')) {
        isError = true;
        const { url } = message.location();
        const type = message.type();
        const text = message.text ? message.text() : undefined;
        errors.push({ type, url, text });
      }
    }
    if (log || isError) {
      console.log(`  Message: [${message.type()}] ${message.text()}`);
      if (isError) {
        message.stackTrace().forEach((args) => console.log(`   - ${args.lineNumber}:${args.columnNumber} ${args.url}`));
      }
    }
  });

  console.log('  🟡 Navigating');
  let count = 0;
  do {
    await sleep(1000);
    try {
      const res = await page.goto('http://localhost:3000', {});
      if (res.ok()) {
        break;
      }
    } catch (e) {
      console.log('  ❌ Error', e.message);
    }
    count += 1;
  } while (count < 5);
  if (count > 5) {
    await browser.close();
    throw new Error('Failed to navigate');
  } else {
    console.log('  🟡 Navigated');
  }

  // wait for start output
  await page.waitForSelector('#starting');

  if (errors.find((e) => e.url.includes('worker'))) {
    console.log('  ❌ Error - worker did not load');
    return errors;
  }
  if (errors.length) {
    console.log('  ❌ Error - worker error');
    return errors;
  }

  console.log('  🟢 Started');

  await page.waitForSelector('#text-output', { timeout: TIMEOUT });
  console.log('  🟢 Got text output');
  // await page.waitForSelector('#image-output');
  // console.log('  🟢 Got image output');
  await page.waitForSelector('#done');
  console.log('  🟢 Got done');

  await page.close();
  await browser.close();

  console.log('  ✅ test complete');

  return errors;
}
