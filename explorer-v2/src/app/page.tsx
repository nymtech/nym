import { WorldMap } from "@/components/worldMap/WorldMap";
import { Stack } from "@mui/material";
import Grid from "@mui/material/Grid2";
import BlogArticlesCards from "../components/blogs/BlogArticleCards";
import { ContentLayout } from "../components/contentLayout/ContentLayout";
import SectionHeading from "../components/headings/SectionHeading";
import { CurrentEpochCard } from "../components/landingPageComponents/CurrentEpochCard";
import { NetworkStakeCard } from "../components/landingPageComponents/NetworkStakeCard";
import { NoiseCard } from "../components/landingPageComponents/NoiseCard";
import { StakersNumberCard } from "../components/landingPageComponents/StakersNumberCard";
import { TokenomicsCard } from "../components/landingPageComponents/TokenomicsCard";
import NodeTable from "../components/nodeTable/NodeTableWithAction";
import NodeAndAddressSearch from "../components/search/NodeAndAddressSearch";

export default async function Home() {
  return (
    <ContentLayout>
      <Grid container columnSpacing={5} rowSpacing={5}>
        <WorldMap />
      </Grid>

      <Stack gap={5}>
        <NodeAndAddressSearch />
      </Stack>
      <Grid container columnSpacing={5} rowSpacing={5}>
        <Grid size={12}>
          <SectionHeading title="Noise Generating Network Overview" />
        </Grid>
        <Grid size={{ xs: 12, sm: 6, lg: 3 }}>
          <NoiseCard />
        </Grid>
        <Grid
          container
          columnSpacing={5}
          rowSpacing={5}
          size={{ xs: 12, sm: 6, lg: 3 }}
        >
          <Grid size={12}>
            <StakersNumberCard />
          </Grid>
          <Grid size={12}>
            <CurrentEpochCard />
          </Grid>
        </Grid>

        <Grid size={{ xs: 12, sm: 6, lg: 3 }}>
          <NetworkStakeCard />
        </Grid>
        <Grid size={{ xs: 12, sm: 6, lg: 3 }}>
          <TokenomicsCard />
        </Grid>
      </Grid>
      <Grid container>
        <Grid size={12}>
          <SectionHeading title="Nym Nodes" />
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
