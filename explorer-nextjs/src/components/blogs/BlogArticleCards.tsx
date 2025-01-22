import fs from "node:fs/promises";
import path from "node:path";
import Grid from "@mui/material/Grid2";
import ExplorerHeroCard from "../cards/ExplorerHeroCard";
import type { BlogArticleWithLink } from "./types";

// TODO: Articles should be sorted by date

const BlogArticlesCards = async ({ limit }: { limit?: number }) => {
  const blogsDir = path.join(process.cwd(), "/src/data");
  const blogsDirFilenames = await fs.readdir(blogsDir);

  // Read all blog articles from the data directory
  const blogArticles: BlogArticleWithLink[] = await Promise.all(
    blogsDirFilenames.map(async (filename) => {
      const filePath = path.join(blogsDir, filename);
      const fileContent = await fs.readFile(filePath, "utf-8");
      const blogArticle = JSON.parse(fileContent);
      return {
        ...blogArticle,
        link: `/onboarding/${filename.replace(".json", "")}`,
      };
    }),
  );

  const limitedBlogArticles = limit
    ? blogArticles.slice(0, limit)
    : blogArticles;

  return limitedBlogArticles
    .sort((a, b) => {
      // sort by date
      return (
        new Date(b.attributes.date).getTime() -
        new Date(a.attributes.date).getTime()
      );
    })
    .map((blogArticle) => {
      return (
        <Grid
          size={{
            sm: 12,
            md: 6,
          }}
          key={blogArticle.title}
        >
          <ExplorerHeroCard
            label={blogArticle.label}
            title={blogArticle.title}
            description={blogArticle.description}
            icon={blogArticle.icon}
            link={blogArticle.link || ""}
            sx={{ height: "100%" }}
          />
        </Grid>
      );
    });
};

export default BlogArticlesCards;
