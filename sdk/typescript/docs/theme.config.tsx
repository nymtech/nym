import React from 'react';
import { DocsThemeConfig } from 'nextra-theme-docs';
import { Footer } from './components/footer';

const config: DocsThemeConfig = {
  logo: <span>Nym TypeScript SDK</span>,
  project: {
    link: 'https://github.com/nymtech/',
  },
  chat: {
    link: 'https://discord.com',
  },
  docsRepositoryBase: 'https://github.com/shuding/nextra-docs-template',
  footer: {
    text: Footer,
  },
  primaryHue: {
    dark: 30,
    light: 30,
  },
};

export default config;
