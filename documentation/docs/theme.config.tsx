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

    const defaultDescription =
      "Nym is a privacy platform. It provides strong network-level privacy against sophisticated end-to-end attackers, and anonymous access control using blinded, re-randomizable, decentralized credentials.";

    // Frontmatter-first description
    const description = config.frontMatter.description || defaultDescription;

    const baseTitle = config.frontMatter.title || config.title || "";
    const title =
      route === "/"
        ? "Nym Docs: Privacy Network Documentation"
        : baseTitle.includes("| Nym Docs")
        ? baseTitle
        : `${baseTitle} | Nym Docs`;

    const pageUrl = `${url}${route}`;

    // Frontmatter fields
    const section = config.frontMatter.section || "";
    const lastUpdated = config.frontMatter.lastUpdated || "";
    const schemaType = config.frontMatter.schemaType || "TechArticle";

    // JSON-LD structured data
    const org = {
      "@id": "https://nym.com/#org",
      "@type": "Organization",
      name: "Nym Technologies SA",
      url: "https://nym.com",
      logo: {
        "@id": "https://nym.com/#logo",
        "@type": "ImageObject",
        url: "https://nym.com/apple-touch-icon.png",
      },
      sameAs: ["https://x.com/nymproject", "https://github.com/nymtech"],
    };

    const website = {
      "@id": "https://nym.com/docs#website",
      "@type": "WebSite",
      name: "Nym Docs",
      url: "https://nym.com/docs",
      publisher: { "@id": "https://nym.com/#org" },
    };

    const webpage = {
      "@id": `${pageUrl}#webpage`,
      "@type": "WebPage",
      url: pageUrl,
      name: title,
      description: description,
      inLanguage: "en",
      isPartOf: { "@id": "https://nym.com/docs#website" },
      breadcrumb: { "@id": `${pageUrl}#breadcrumb` },
      potentialAction: { "@type": "ReadAction", target: pageUrl },
    };

    const articleSchema: Record<string, any> = {
      "@id": `${pageUrl}#article`,
      "@type": schemaType,
      ...(schemaType === "HowTo"
        ? { name: baseTitle }
        : { headline: baseTitle }),
      description: description,
      url: pageUrl,
      author: { "@id": "https://nym.com/#org" },
      publisher: { "@id": "https://nym.com/#org" },
      mainEntityOfPage: { "@id": `${pageUrl}#webpage` },
      ...(lastUpdated && {
        datePublished: lastUpdated,
        dateModified: lastUpdated,
      }),
    };

    const pathParts = route.split("/").filter(Boolean);
    const breadcrumb = {
      "@id": `${pageUrl}#breadcrumb`,
      "@type": "BreadcrumbList",
      itemListElement: pathParts.map((part: string, i: number) => ({
        "@type": "ListItem",
        position: i + 1,
        name:
          config.frontMatter.breadcrumbLabel && i === pathParts.length - 1
            ? config.frontMatter.breadcrumbLabel
            : part.charAt(0).toUpperCase() + part.slice(1).replace(/-/g, " "),
        item: `${url}/${pathParts.slice(0, i + 1).join("/")}`,
      })),
    };

    const schema = {
      "@context": "https://schema.org",
      "@graph": [org, website, webpage, articleSchema, breadcrumb],
    };

    return (
      <>
        <title>{title}</title>
        <meta name="author" content="Nym" />
        <link rel="canonical" href={pageUrl} />
        <link rel="icon" href={favicon} type="image/svg+xml" />
        <meta name="description" content={description} />
        <meta property="og:title" content={title} />
        <meta property="og:site_name" content="Nym docs" />
        <meta property="og:description" content={description} />
        <meta property="og:image" content={image} />
        <meta property="og:type" content="article" />
        <meta property="og:url" content={pageUrl} />
        <meta property="og:image:width" content="1200" />
        <meta property="og:image:height" content="630" />
        {section && <meta property="article:section" content={section} />}
        {lastUpdated && (
          <meta property="article:modified_time" content={lastUpdated} />
        )}
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
  logo: <span style={{ fontFamily: "var(--font-mono)", fontSize: "1.1rem", fontWeight: 700 }}>Nym Docs</span>,
  project: {
    link: "https://github.com/nymtech/nym",
  },
  docsRepositoryBase:
    "https://github.com/nymtech/nym/tree/develop/documentation/docs/",
  // footer: {
  //   text: Footer,
  // },
  darkMode: true,
  primaryHue: 135,
  primarySaturation: 64,
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
