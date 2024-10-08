import React from "react";
import { DocsThemeConfig } from "nextra-theme-docs";
import { Footer } from "./components/footer";

const config: DocsThemeConfig = {
  logo: <span>Nym Docs</span>,
  project: {
    link: "https://github.com/nymtech/nym",
  },
  chat: {
    link: "https://matrix.to/#/#dev:nymtech.chat",
  },
  docsRepositoryBase:
    "https://github.com/nymtech/nym/tree/develop/documentation/",
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
};

export default config;
