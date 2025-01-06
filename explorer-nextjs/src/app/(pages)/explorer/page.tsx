import BlogArticlesCards from "@/components/blogs/BlogArticleCards";
import CardSkeleton from "@/components/cards/Skeleton";
import { ContentLayout } from "@/components/contentLayout/ContentLayout";
import SectionHeading from "@/components/headings/SectionHeading";
import NodeTableWithAction from "@/components/nodeTable/NodeTableWithAction";
import { Wrapper } from "@/components/wrapper";
import Grid from "@mui/material/Grid2";
import { Suspense } from "react";

export default function ExplorerPage() {
  return (
    <ContentLayout>
      <Wrapper>
        <SectionHeading title="Explorer" />
        <Suspense fallback={<CardSkeleton sx={{ mt: 5 }} />}>
          <NodeTableWithAction />
        </Suspense>
        <Grid container columnSpacing={5} rowSpacing={5} mt={10}>
          <Grid size={12}>
            <SectionHeading title="Onboarding" />
          </Grid>
          <BlogArticlesCards limit={2} />
        </Grid>
      </Wrapper>
    </ContentLayout>
  );
}
