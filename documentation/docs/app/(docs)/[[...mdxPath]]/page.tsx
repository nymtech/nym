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
  const { default: MDXContent, ...rest } = result;
  const jsonLd = buildDocsJsonLd(result.metadata, params.mdxPath);
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
