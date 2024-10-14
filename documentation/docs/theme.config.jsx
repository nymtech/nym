import React from "react";
import { DocsThemeConfig } from "nextra-theme-docs";
import { Search } from 'nextra-theme-docs';
import { useConfig } from 'nextra-theme-docs';


import { Footer } from "./components/footer";
import {Matrix} from "./components/matrix-link";

const config = {
  logo: <span>Nym Docs</span>,
  project: {
    link: "https://github.com/nymtech/nym",
  },
  chat: {
    link: "https://matrix.to/#/#dev:nymtech.chat",
  },
  docsRepositoryBase:
    "https://github.com/nymtech/nym/tree/develop/documentation/docs",
  footer: {
    text: Footer,
  },
  darkMode: true,
  nextThemes: {
    forcedTheme: "dark",
  },
  primaryHue: {
    dark: 30,
    light: 30,
  },
  sidebar: {
    defaultMenuCollapseLevel: 1,
    autoCollapse: true,
  },
  navbar : { 
    extraContent: <Matrix/>
}
};

export default config;
