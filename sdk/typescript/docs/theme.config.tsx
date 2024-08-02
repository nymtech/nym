import React from 'react';
import { DocsThemeConfig } from 'nextra-theme-docs';
import { Footer } from './components/footer';

const config: DocsThemeConfig = {
  logo: <span>Nym TypeScript SDK</span>,
  project: {
    link: 'https://github.com/nymtech/nym',
  },
  chat: {
    link: 'https://nymtech.net/go/discord',
  },
  docsRepositoryBase: 'https://github.com/nymtech/nym/tree/develop/sdk/typescript/docs',
  footer: {
    text: Footer,
  },
  darkMode: false,
  nextThemes: {
    forcedTheme: 'dark',
  },
  primaryHue: {
    dark: 30,
    light: 30,
  },
};

export default config;
