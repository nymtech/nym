import BlogArticlesCards from "@/components/blogs/BlogArticleCards";
import { ContentLayout } from "@/components/contentLayout/ContentLayout";
import SectionHeading from "@/components/headings/SectionHeading";
import Grid from "@mui/material/Grid2";

export default function OnboardingPage() {
  return (
    <ContentLayout>
      <SectionHeading title="Onboarding page" />
      <Grid container spacing={4}>
        <BlogArticlesCards />
      </Grid>
    </ContentLayout>
  );
}
