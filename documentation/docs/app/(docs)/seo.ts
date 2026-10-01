import type { Metadata } from "next";

// Per-page SEO, ported from the old theme.config.tsx `head` function. Static and derived
// tags go through generateMetadata (buildDocsMetadata); the schema.org JSON-LD graph is
// rendered as a <script> in the page (buildDocsJsonLd), since the Metadata API has no slot
// for arbitrary <script> content.

interface DocsFrontMatter {
  title?: string | null;
  description?: string | null;
  section?: string | null;
  lastUpdated?: string | null;
  schemaType?: string | null;
  breadcrumbLabel?: string | null;
}

const SITE_TITLE = "Nym Docs";
const ROOT_TITLE = "Nym Docs: Privacy Network Documentation";
const DEFAULT_DESCRIPTION =
  "Nym is a privacy platform. It provides strong network-level privacy against sophisticated end-to-end attackers, and anonymous access control using blinded, re-randomizable, decentralized credentials.";

// NEXT_PUBLIC_SITE_URL already carries the /docs basePath in production, matching the old
// theme.config head().
function siteUrl() {
  return process.env.NEXT_PUBLIC_SITE_URL || "https://nym.com/docs";
}

function routeFromMdxPath(mdxPath: string[] = []) {
  return `/${mdxPath.join("/")}`;
}

function resolveTitle(route: string, frontMatter: DocsFrontMatter) {
  const baseTitle = frontMatter.title || "";
  const title =
    route === "/"
      ? ROOT_TITLE
      : baseTitle.includes(`| ${SITE_TITLE}`)
        ? baseTitle
        : `${baseTitle} | ${SITE_TITLE}`;
  return { title, baseTitle };
}

export function buildDocsMetadata(
  frontMatter: DocsFrontMatter,
  mdxPath: string[] = [],
): Metadata {
  const url = siteUrl();
  const route = routeFromMdxPath(mdxPath);
  const { title } = resolveTitle(route, frontMatter);
  const description = frontMatter.description || DEFAULT_DESCRIPTION;
  const pageUrl = route === "/" ? url : `${url}${route}`;
  const ogImage = `${url}/images/Nym_meta_Image.png`;

  return {
    // absolute bypasses the root layout's title template: the suffix and root-route case
    // are already applied here.
    title: { absolute: title },
    description,
    authors: [{ name: "Nym" }],
    alternates: { canonical: pageUrl },
    openGraph: {
      title,
      description,
      siteName: "Nym docs",
      type: "article",
      url: pageUrl,
      images: [{ url: ogImage, width: 1200, height: 630 }],
      ...(frontMatter.section && { section: frontMatter.section }),
      ...(frontMatter.lastUpdated && { modifiedTime: frontMatter.lastUpdated }),
    },
    twitter: {
      card: "summary_large_image",
      site: "@nymproject",
      title,
      description,
      images: [ogImage],
    },
  };
}

export function buildDocsJsonLd(
  frontMatter: DocsFrontMatter,
  mdxPath: string[] = [],
) {
  const url = siteUrl();
  const route = routeFromMdxPath(mdxPath);
  const { title, baseTitle } = resolveTitle(route, frontMatter);
  const description = frontMatter.description || DEFAULT_DESCRIPTION;
  const pageUrl = route === "/" ? url : `${url}${route}`;
  const schemaType = frontMatter.schemaType || "TechArticle";
  const lastUpdated = frontMatter.lastUpdated || "";

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
    name: SITE_TITLE,
    url: "https://nym.com/docs",
    publisher: { "@id": "https://nym.com/#org" },
  };

  const webpage = {
    "@id": `${pageUrl}#webpage`,
    "@type": "WebPage",
    url: pageUrl,
    name: title,
    description,
    inLanguage: "en",
    isPartOf: { "@id": "https://nym.com/docs#website" },
    breadcrumb: { "@id": `${pageUrl}#breadcrumb` },
    potentialAction: { "@type": "ReadAction", target: pageUrl },
  };

  const article: Record<string, unknown> = {
    "@id": `${pageUrl}#article`,
    "@type": schemaType,
    ...(schemaType === "HowTo" ? { name: baseTitle } : { headline: baseTitle }),
    description,
    url: pageUrl,
    author: { "@id": "https://nym.com/#org" },
    publisher: { "@id": "https://nym.com/#org" },
    mainEntityOfPage: { "@id": `${pageUrl}#webpage` },
    ...(lastUpdated && { datePublished: lastUpdated, dateModified: lastUpdated }),
  };

  const pathParts = route.split("/").filter(Boolean);
  const breadcrumb = {
    "@id": `${pageUrl}#breadcrumb`,
    "@type": "BreadcrumbList",
    itemListElement: pathParts.map((part, i) => ({
      "@type": "ListItem",
      position: i + 1,
      name:
        frontMatter.breadcrumbLabel && i === pathParts.length - 1
          ? frontMatter.breadcrumbLabel
          : part.charAt(0).toUpperCase() + part.slice(1).replace(/-/g, " "),
      item: `${url}/${pathParts.slice(0, i + 1).join("/")}`,
    })),
  };

  return {
    "@context": "https://schema.org",
    "@graph": [org, website, webpage, article, breadcrumb],
  };
}
