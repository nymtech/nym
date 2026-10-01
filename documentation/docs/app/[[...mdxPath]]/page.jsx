import { generateStaticParamsFor, importPage } from "nextra/pages";
import { useMDXComponents as getMDXComponents } from "../../mdx-components";

export const generateStaticParams = generateStaticParamsFor("mdxPath");

// Matches the old theme.config head(): NEXT_PUBLIC_SITE_URL is the absolute base and
// already carries the /docs basePath in production.
const siteUrl = process.env.NEXT_PUBLIC_SITE_URL || "https://nym.com/docs";

function routeOf(mdxPath) {
  return "/" + (mdxPath ?? []).join("/");
}

function pageUrlOf(mdxPath) {
  const route = routeOf(mdxPath);
  return route === "/" ? siteUrl : `${siteUrl}${route}`;
}

export async function generateMetadata(props) {
  const params = await props.params;
  const { metadata = {} } = await importPage(params.mdxPath);
  const route = routeOf(params.mdxPath);
  const pageUrl = pageUrlOf(params.mdxPath);

  const title =
    route === "/"
      ? { absolute: "Nym Docs: Privacy Network Documentation" }
      : metadata.title;

  return {
    ...(title && { title }),
    ...(metadata.description && { description: metadata.description }),
    alternates: { canonical: pageUrl },
    openGraph: {
      url: pageUrl,
      ...(metadata.title && { title: metadata.title }),
      ...(metadata.description && { description: metadata.description }),
      ...(metadata.section && { section: metadata.section }),
      ...(metadata.lastUpdated && { modifiedTime: metadata.lastUpdated }),
    },
  };
}

// Full JSON-LD graph, ported from the old theme.config head(). Built per route from
// the path and frontmatter (section, lastUpdated, schemaType, breadcrumbLabel).
function buildJsonLd(mdxPath, metadata = {}) {
  const route = routeOf(mdxPath);
  const pageUrl = pageUrlOf(mdxPath);
  const baseTitle = metadata.title || "";
  const title =
    route === "/" ? "Nym Docs: Privacy Network Documentation" : baseTitle;
  const description =
    metadata.description ||
    "Nym is a privacy platform. It provides strong network-level privacy against sophisticated end-to-end attackers, and anonymous access control using blinded, re-randomizable, decentralized credentials.";
  const schemaType = metadata.schemaType || "TechArticle";

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
    description,
    inLanguage: "en",
    isPartOf: { "@id": "https://nym.com/docs#website" },
    breadcrumb: { "@id": `${pageUrl}#breadcrumb` },
    potentialAction: { "@type": "ReadAction", target: pageUrl },
  };

  const articleSchema = {
    "@id": `${pageUrl}#article`,
    "@type": schemaType,
    ...(schemaType === "HowTo" ? { name: baseTitle } : { headline: baseTitle }),
    description,
    url: pageUrl,
    author: { "@id": "https://nym.com/#org" },
    publisher: { "@id": "https://nym.com/#org" },
    mainEntityOfPage: { "@id": `${pageUrl}#webpage` },
    ...(metadata.lastUpdated && {
      datePublished: metadata.lastUpdated,
      dateModified: metadata.lastUpdated,
    }),
  };

  const pathParts = route.split("/").filter(Boolean);
  const breadcrumb = {
    "@id": `${pageUrl}#breadcrumb`,
    "@type": "BreadcrumbList",
    itemListElement: pathParts.map((part, i) => ({
      "@type": "ListItem",
      position: i + 1,
      name:
        metadata.breadcrumbLabel && i === pathParts.length - 1
          ? metadata.breadcrumbLabel
          : part.charAt(0).toUpperCase() + part.slice(1).replace(/-/g, " "),
      item: `${siteUrl}/${pathParts.slice(0, i + 1).join("/")}`,
    })),
  };

  return {
    "@context": "https://schema.org",
    "@graph": [org, website, webpage, articleSchema, breadcrumb],
  };
}

const Wrapper = getMDXComponents().wrapper;

export default async function Page(props) {
  const params = await props.params;
  const result = await importPage(params.mdxPath);
  // Spread the whole importPage result (toc, metadata, sourceCode, ...) onto the
  // Wrapper. Nextra's wrapper reads several of these; passing only toc+metadata makes
  // it throw during render.
  const { default: MDXContent, ...rest } = result;
  const jsonLd = buildJsonLd(params.mdxPath, rest.metadata);

  return (
    <Wrapper {...rest}>
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{ __html: JSON.stringify(jsonLd) }}
      />
      <MDXContent {...props} params={params} />
    </Wrapper>
  );
}
