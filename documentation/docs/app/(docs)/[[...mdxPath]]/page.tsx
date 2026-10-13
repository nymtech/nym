import { generateStaticParamsFor, importPage } from "nextra/pages";
import { useMDXComponents } from "../../../mdx-components";
import { buildDocsJsonLd, buildDocsMetadata } from "../seo";

// Catch-all that renders the content/** MDX tree. The file path under content/ is the route,
// so every current URL is preserved.

export const generateStaticParams = generateStaticParamsFor("mdxPath");

export async function generateMetadata(props: {
  params: Promise<{ mdxPath: string[] }>;
}) {
  const params = await props.params;
  const { metadata } = await importPage(params.mdxPath);
  return buildDocsMetadata(metadata, params.mdxPath);
}

const Wrapper = useMDXComponents().wrapper;

export default async function Page(props: {
  params: Promise<{ mdxPath: string[] }>;
}) {
  const params = await props.params;
  const result = await importPage(params.mdxPath);
  // Not {...result}: an MDX page's own `export const` values would land on the wrapper's
  // <div> as unknown DOM props. sourceCode is left out because the theme's copy button is off.
  const { default: MDXContent, toc, metadata } = result;
  const jsonLd = buildDocsJsonLd(metadata, params.mdxPath);
  return (
    <Wrapper toc={toc} metadata={metadata}>
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{ __html: JSON.stringify(jsonLd) }}
      />
      <MDXContent {...props} params={params} />
    </Wrapper>
  );
}
