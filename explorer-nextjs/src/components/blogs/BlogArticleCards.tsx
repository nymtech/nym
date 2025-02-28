import fs from "node:fs/promises";
import path from "node:path";
import Grid from "@mui/material/Grid2";
import ExplorerHeroCard from "../cards/ExplorerHeroCard";
import type { BlogArticleWithLink } from "./types";
import { icons, IconName } from "@/utils/getIconByName";

// TODO: Articles should be sorted by date

const BlogArticlesCards = async ({
  limit,
  ids,
}: {
  limit?: number;
  ids?: Array<number>;
}) => {
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

  const limitedOrFilteredBlogArticles = (
    blogArticles: BlogArticleWithLink[],
    limit?: number,
    ids?: number[],
  ): BlogArticleWithLink[] => {
    let filteredArticles = blogArticles;

    // Filter by IDs if provided
    if (ids && ids.length > 0) {
      filteredArticles = filteredArticles.filter((article) =>
        ids.includes(article.id),
      );
    }

    // Apply limit if provided
    if (limit) {
      filteredArticles = filteredArticles.slice(0, limit);
    }

    return filteredArticles;
  };
  const articles = limitedOrFilteredBlogArticles(blogArticles, limit, ids);

  return articles
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
            icon={icons[blogArticle.icon as IconName]?.src}
            link={blogArticle.link || ""}
            sx={{ height: "100%" }}
          />
        </Grid>
      );
    });
};

export default BlogArticlesCards;
