import React from "react";
import { DocsThemeConfig } from "nextra-theme-docs";
import { Footer } from "./components/footer";
import { Matrix } from "./components/matrix-link";

const config: DocsThemeConfig = {
  logo: <span>Nym Docs</span>,
  project: {
    link: "https://github.com/nymtech/nym",
  },
  docsRepositoryBase:
    "https://github.com/nymtech/nym/tree/develop/documentation/docs/",
  footer: {
    text: Footer,
  },
  darkMode: true,
  primaryHue: {
    dark: 30,
    light: 30,
  },
  primarySaturation: 68,
  sidebar: {
    defaultMenuCollapseLevel: 1,
    autoCollapse: true,
    toggleButton: true,
  },
  navbar: {
    extraContent: <Matrix />,
  },
  toc: {
    float: false,
  },
  // gitTimestamp: TODO ,
  editLink: {
    component: null,
  },
  feedback: {
    content: null,
  },
};

export default config;
