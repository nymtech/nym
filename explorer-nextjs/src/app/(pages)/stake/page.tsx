import BlogArticlesCards from "@/components/blogs/BlogArticleCards";
import Grid from "@mui/material/Grid2";
import { ContentLayout } from "../../../components/contentLayout/ContentLayout";
import SectionHeading from "../../../components/headings/SectionHeading";
import OverviewCards from "../../../components/staking/OverviewCards";
import StakeTableWithAction from "../../../components/staking/StakeTableWithAction";
import SubHeaderRow from "../../../components/staking/SubHeaderRow";

export default async function StakingPage() {
  return (
    <ContentLayout>
      <SectionHeading title="Staking" />
      <SubHeaderRow />
      <OverviewCards />
      <StakeTableWithAction />
      <Grid container columnSpacing={5} rowSpacing={5}>
        <Grid size={12}>
          <SectionHeading title="Onboarding" />
        </Grid>
        <BlogArticlesCards ids={[2]} />
      </Grid>
    </ContentLayout>
  );
}
