import React from "react";
import { DocsThemeConfig, useConfig } from "nextra-theme-docs";
import { Footer } from "./components/footer";
import { Matrix } from "./components/matrix-link";
import { useRouter } from "next/router";

const config: DocsThemeConfig = {
  head: function useHead() {
    const config = useConfig()
    const { route } = useRouter()
    const url = process.env.NEXT_PUBLIC_SITE_URL
    const image = url + '/nym_logo.jpg'

    const description =
      config.frontMatter.description ||
      'Nym is a privacy platform. It provides strong network-level privacy against sophisticated end-to-end attackers, and anonymous access control using blinded, re-randomizable, decentralized credentials.'
    const title = config.title + (route === '/' ? '' : ' - Nym docs')

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
