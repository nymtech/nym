import React from "react";
import { DocsThemeConfig, useConfig } from "nextra-theme-docs";
// import { Footer } from "./components/footer";
import { Matrix } from "./components/matrix-link";
import { Explorer } from "./components/explorer-link";
import { useRouter } from "next/router";

const config: DocsThemeConfig = {
  head: function useHead() {
    const config = useConfig();
    const { route } = useRouter();
    const url = process.env.NEXT_PUBLIC_SITE_URL;
    const image = url + "/nym_logo.jpg";

    // Define descriptions for different "books"
    const bookDescriptions: Record<string, string> = {
      "/developers":
        "Nym's developer documentation covering core concepts of integrating with the Mixnet, interacting with the Nyx blockchain, an overview of the avaliable tools, and our SDK docs.",
      "/network":
        "Nym's network documentation covering network architecture, node types, tokenomics, and cryptography.",
      "/operators":
        "Nym's Operators guide containing information and setup guides for the various components of Nym network and Nyx blockchain validators.",
      "/apis":
        "Interactive APIs generated from the OpenAPI specs of various API endpoints offered by bits of Nym infrastructure run both by Nym and community operators for both Mainnet and the Sandbox testnet.",
    };

    const defaultDescription =
      "Nym is a privacy platform. It provides strong network-level privacy against sophisticated end-to-end attackers, and anonymous access control using blinded, re-randomizable, decentralized credentials.";

    const topLevel = "/" + route.split("/")[1];
    const description =
      config.frontMatter.description ||
      bookDescriptions[topLevel] ||
      defaultDescription;

    const title = config.title + (route === "/" ? "" : " - Nym docs");

    return (
      <>
        <title>{title}</title>
        <meta name="author" content="Nym" />
        <link rel="canonical" href={url + route} />

        <meta property="og:title" content={title} />
        <meta property="og:site_name" content="Nym docs"></meta>
        <meta name="description" content={description} />
        <meta property="og:description" content={description} />
        <meta property="og:image" content={image} />
        <meta property="og:type" content="website" />
        <meta property="og:url" content={url + route}></meta>

        <meta property="twitter:title" content={title}></meta>
        <meta property="twitter:description" content={description}></meta>
        <meta name="twitter:card" content="summary_large_image" />
        <meta property="twitter:image" content={image}></meta>
        <meta name="twitter:site" content="@nymproject" />
        <meta name="twitter:site:domain" content={url} />
        <meta name="twitter:url" content={url + route} />

        <meta name="apple-mobile-web-app-title" content="Nym docs" />
      </>
    );
  },
  logo: <span>Nym Docs</span>,
  project: {
    link: "https://github.com/nymtech/nym",
  },
  docsRepositoryBase:
    "https://github.com/nymtech/nym/tree/develop/documentation/docs/",
  // footer: {
  //   text: Footer,
  // },
  darkMode: true,
  nextThemes: {
    defaultTheme: "dark",
  },
  sidebar: {
    defaultMenuCollapseLevel: 1,
    autoCollapse: true,
  },

  navbar: {
    extraContent: (
      <>
        <Matrix />
        <Explorer />
      </>
    ),
  },
  toc: {
    float: false,
    component: null,
  },
  editLink: {
    component: null, // remove element
  },
  feedback: {
    content: null, // remove element
  },
  // gitTimestamp: TODO
};

export default config;
