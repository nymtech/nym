import React from "react";
import { DocsThemeConfig } from "nextra-theme-docs";
import { Footer } from "./components/footer";
import { Matrix } from "./components/matrix-link";
import { useRouter } from "next/router";

const config: DocsThemeConfig = {
  useNextSeoProps() {
    return {
      titleTemplate: "%s – Nym Docs",
    };
  },
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
    // if we do this then we also have to uncomment the editLink and feedback objects below
  },
  // editLink: {
  //   component: null,
  // },
  // feedback: {
  //   content: null,
  // },

  // gitTimestamp: TODO
};

export default config;
