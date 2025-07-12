import { WorldMap } from "@/components/worldMap/WorldMap";
import { Stack } from "@mui/material";
import Grid from "@mui/material/Grid2";
import BlogArticlesCards from "../components/blogs/BlogArticleCards";
import { ContentLayout } from "../components/contentLayout/ContentLayout";
import SectionHeading from "../components/headings/SectionHeading";
import { CurrentEpochCardWrapper } from "../components/landingPageComponents/CurrentEpochCardWrapper";
import { NetworkStakeCardWrapper } from "../components/landingPageComponents/NetworkStakeCardWrapper";
import { NoiseCardWrapper } from "../components/landingPageComponents/NoiseCardWrapper";
import { StakersNumberCardWrapper } from "../components/landingPageComponents/StakersNumberCardWrapper";
import { TokenomicsCardWrapper } from "../components/landingPageComponents/TokenomicsCardWrapper";
import NodeTable from "../components/nodeTable/NodeTableWithAction";
import NodeAndAddressSearch from "../components/search/NodeAndAddressSearch";

export default async function Home() {
  return (
    <ContentLayout>
      <Stack gap={5}>
        <NodeAndAddressSearch />
        <WorldMap />
      </Stack>
      <Grid container columnSpacing={5} rowSpacing={5}>
        <Grid size={12}>
          <SectionHeading title="Network Overview" />
        </Grid>
        <NoiseCardWrapper />
        <Grid
          container
          columnSpacing={5}
          rowSpacing={5}
          size={{ xs: 12, sm: 6, lg: 3 }}
        >
          <StakersNumberCardWrapper />
          <CurrentEpochCardWrapper />
        </Grid>

        <NetworkStakeCardWrapper />
        <TokenomicsCardWrapper />
      </Grid>
      <Grid container rowSpacing={5}>
        <Grid size={12}>
          <SectionHeading title="Nym Servers" />
        </Grid>
        <Grid size={12}>
          <NodeTable />
        </Grid>
      </Grid>
      <Grid container columnSpacing={5} rowSpacing={5}>
        <Grid size={12}>
          <SectionHeading title="Onboarding" />
        </Grid>
        <BlogArticlesCards ids={[1, 2]} />
      </Grid>
    </ContentLayout>
  );
}
