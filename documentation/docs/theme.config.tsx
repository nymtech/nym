
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
    const image = url + "/images/Nym_meta_Image.png";
    const favicon = url + "/favicon.svg";

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
    const title = route === "/" ? "Nym docs" : config.title + " - Nym docs";
    const pageUrl = url + route;

    const section = config.frontMatter?.section || "";
    const lastUpdated = config.frontMatter?.lastUpdated || "";
    const schemaType = config.frontMatter?.schemaType || "TechArticle";

    const org = {
      "@id": `${url}/#org`,
      "@type": "Organization",
      "name": "Nym Technologies SA",
      "url": url,
      "logo": {
        "@id": `${url}/#logo`,
        "@type": "ImageObject",
        "url": `${url}/apple-touch-icon.png`
      },
      "sameAs": ["https://x.com/nymproject", "https://github.com/nymtech"]
    };

    const website = {
      "@id": `${url}/docs#website`,
      "@type": "WebSite",
      "name": "Nym Docs",
      "url": `${url}/docs`,
      "publisher": { "@id": `${url}/#org` }
    };

    const webpage = {
      "@id": `${pageUrl}#webpage`,
      "@type": "WebPage",
      "url": pageUrl,
      "name": title,
      "description": description,
      "inLanguage": "en",
      "isPartOf": { "@id": `${url}/docs#website` },
      "breadcrumb": { "@id": `${pageUrl}#breadcrumb` },
      "potentialAction": { "@type": "ReadAction", "target": pageUrl }
    };

    const articleSchema = {
      "@id": `${pageUrl}#article`,
      "@type": schemaType,
      ...(schemaType === "HowTo" ? { "name": config.title } : { "headline": config.title }),
      "description": description,
      "url": pageUrl,
      "author": { "@id": `${url}/#org` },
      "publisher": { "@id": `${url}/#org` },
      "mainEntityOfPage": { "@id": `${pageUrl}#webpage` },
      ...(lastUpdated && {
        "datePublished": lastUpdated,
        "dateModified": lastUpdated
      })
    };

    const pathParts = route.split('/').filter(Boolean);
    const breadcrumb = {
      "@id": `${pageUrl}#breadcrumb`,
      "@type": "BreadcrumbList",
      itemListElement: pathParts.map((part, i) => ({
        "@type": "ListItem",
        position: i + 1,
        name: part.charAt(0).toUpperCase() + part.slice(1).replace(/-/g, ' '),
        item: `${url}/${pathParts.slice(0, i + 1).join('/')}`
      }))
    };

    const schema = {
      "@context": "https://schema.org",
      "@graph": [org, website, webpage, articleSchema, breadcrumb]
    };

    return (
      <>
        <title>{title}</title>
        <meta name="author" content="Nym" />
        <link rel="canonical" href={pageUrl} />
        <link rel="icon" href={favicon} type="image/svg+xml" />
        <meta property="og:title" content={title} />
        <meta property="og:site_name" content="Nym docs" />
        <meta name="description" content={description} />
        <meta property="og:description" content={description} />
        <meta property="og:image" content={image} />
        <meta property="og:type" content="article" />
        <meta property="og:url" content={pageUrl} />
        {section && <meta property="article:section" content={section} />}
        {lastUpdated && <meta property="article:modified_time" content={lastUpdated} />}
        <meta property="twitter:title" content={title} />
        <meta property="twitter:description" content={description} />
        <meta name="twitter:card" content="summary_large_image" />
        <meta property="twitter:image" content={image} />
        <meta name="twitter:site" content="@nymproject" />
        <meta name="twitter:site:domain" content={url} />
        <meta name="twitter:url" content={pageUrl} />
        <meta name="apple-mobile-web-app-title" content="Nym docs" />
        <script
          type="application/ld+json"
          dangerouslySetInnerHTML={{ __html: JSON.stringify(schema) }}
        />
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
        <Explorer />
        <Matrix />
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
