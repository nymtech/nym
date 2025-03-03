import TableOfContents from "@/components/blogs/TableOfContents";
import type BlogArticle from "@/components/blogs/types";
import { Breadcrumbs } from "@/components/breadcrumbs/Breadcrumbs";
import { ContentLayout } from "@/components/contentLayout/ContentLayout";
import SectionHeading from "@/components/headings/SectionHeading";
import { Link } from "@/components/muiLink";
import { Wrapper } from "@/components/wrapper";
import { Box, Stack, Typography } from "@mui/material";
import Grid from "@mui/material/Grid2";
import { format } from "date-fns";
import Image from "next/image";
import Markdown from "react-markdown";

export default async function BlogPage({
  params,
}: {
  params: Promise<{ slug: string }>;
}) {
  const { slug } = await params;

  try {
    const blogArticle: BlogArticle = await import(`@/data/${slug}.json`);

    const breadcrumbItems = [
      {
        label: "Onboarding",
        href: "/onboarding",
      },
      { label: blogArticle.title, isCurrentPage: true },
    ];
    return (
      <ContentLayout>
        <Wrapper>
          <Grid container spacing={5}>
            <Grid size={{ xs: 12, md: 8 }}>
              <Stack spacing={4}>
                <Breadcrumbs items={breadcrumbItems} />
                <SectionHeading title={blogArticle.title} />
                <Box
                  sx={{
                    borderTop: "1px dashed",
                    paddingBlockStart: "10px",
                    display: "flex",
                    justifyContent: "space-between",
                    alignItems: "center",
                  }}
                >
                  <Typography
                    variant="subtitle3"
                    sx={{
                      display: "flex",
                      gap: "20px",
                      alignItems: "center",
                    }}
                  >
                    <Box
                      sx={{
                        display: "flex",
                        gap: "10px",
                        alignItems: "center",
                      }}
                    >
                      Author
                      {(blogArticle?.attributes?.blogAuthors?.length ?? 0) > 1
                        ? "s"
                        : ""}
                      :{" "}
                      {blogArticle?.attributes?.blogAuthors?.map(
                        (author: string) => (
                          <Typography key={author} variant="subtitle3">
                            {author}
                          </Typography>
                        )
                      )}
                    </Box>
                    <time dateTime={blogArticle?.attributes?.date.toString()}>
                      {format(
                        new Date(blogArticle?.attributes?.date),
                        "MMMM dd, yyyy"
                      )}
                    </time>
                  </Typography>
                  <Typography variant="subtitle3">
                    {blogArticle.attributes.readingTime}{" "}
                    {blogArticle.attributes.readingTime > 1 ? "mins" : "min"}{" "}
                    read
                  </Typography>
                </Box>
                <Image
                  src={blogArticle.image}
                  alt="blog-image"
                  width={120}
                  height={60}
                  sizes="100vw"
                  style={{
                    width: "100%",
                    height: "auto",
                  }}
                />
                <Box>
                  {blogArticle.overview.content.map(({ text }) => (
                    <Box key={text} sx={{ mt: 3 }}>
                      <Typography variant="body2" component="span">
                        <Markdown className="reactMarkDownLink reactMarkDownList">
                          {text}
                        </Markdown>
                      </Typography>
                    </Box>
                  ))}
                </Box>
                {blogArticle.sections.map((section) => (
                  <Box key={section.heading} id={section.id}>
                    <SectionHeading title={section.heading} />
                    {section.text.map(({ text }) => (
                      <Box key={text} sx={{ mt: 3 }}>
                        <Typography variant="body2" component="span">
                          <Markdown className="reactMarkDownLink reactMarkDownList">
                            {text}
                          </Markdown>
                        </Typography>
                      </Box>
                    ))}
                  </Box>
                ))}
              </Stack>
            </Grid>
            <Grid size={{ md: 4 }}>
              <TableOfContents
                headings={blogArticle.sections.map((section) => ({
                  heading: section.heading,
                  id: section.id,
                }))}
              />
            </Grid>
          </Grid>
        </Wrapper>
      </ContentLayout>
    );
  } catch (error) {
    console.log(error);

    return (
      <ContentLayout>
        <Wrapper>
          <SectionHeading title={"Off the grid, like your data"} />
          <Typography variant="body2">
            Oops! Looks like the page you’re looking for got mixed up in the
            noise. Don’t worry, your privacy is intact. Let’s get you
            <Link href="/">back to the homepage.</Link>
          </Typography>
        </Wrapper>
      </ContentLayout>
    );
  }
}
