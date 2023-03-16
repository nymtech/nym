import * as puppeteer from 'puppeteer';

const defaultOptions = {
  headless: true,
};

export default async (options = undefined) => {
  const puppeterOptions = options === undefined ? defaultOptions : options;
  return await puppeteer.launch(puppeterOptions);
};
