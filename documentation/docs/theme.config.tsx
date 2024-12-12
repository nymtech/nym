import React from "react";
import { DocsThemeConfig, useConfig } from "nextra-theme-docs";
import { Footer } from "./components/footer";
import { Matrix } from "./components/matrix-link";
import { useRouter } from "next/router";

const config: DocsThemeConfig = {
  head: function useHead() {
    const config = useConfig()
    const { route } = useRouter()
    const isDefault = route === '/' || !config.title
    const image = 'https://nymtech.net/nym_logo.jpg'

    const description =
      config.frontMatter.description ||
      'Join the privacy ecosystem'
    const title = config.title + (route === '/' ? '' : ' - Nym docs')

    return (
      <>
        <title>{title}</title>
        <meta property="og:title" content={title} />
        <meta name="description" content={description} />
        <meta property="og:description" content={description} />
        <meta property="og:image" content={image} />

        <meta name="msapplication-TileColor" content="#fff" />
        <meta httpEquiv="Content-Language" content="en" />
        <meta name="twitter:card" content="summary_large_image" />
        <meta name="twitter:site:domain" content="nym.com" />
        <meta name="twitter:url" content={"https://nym.com/" + route} />
        <meta name="apple-mobile-web-app-title" content="Nextra" />
        <link rel="icon" href="/favicon.svg" type="image/svg+xml" />
        <link rel="icon" href="/favicon.png" type="image/png" />
        <link
          rel="icon"
          href="/favicon-dark.svg"
          type="image/svg+xml"
          media="(prefers-color-scheme: dark)"
        />
        <link
          rel="icon"
          href="/favicon-dark.png"
          type="image/png"
          media="(prefers-color-scheme: dark)"
        />
      </>
    )
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
