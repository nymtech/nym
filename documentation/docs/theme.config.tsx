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
  sidebar: {
    defaultMenuCollapseLevel: 1,
    autoCollapse: true,
  },
  navbar: {
    extraContent: <Matrix />,
  },
  toc: {
    float: true, // TODO would be nice to set this to false so the TOC is in the left sidebar but this doesn't seem to work with pages that are also the top of directories: fix
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
