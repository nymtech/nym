import ExplorerHeroCard from "@/components/cards/ExplorerHeroCard";
import CardSkeleton from "@/components/cards/Skeleton";
import { ContentLayout } from "@/components/contentLayout/ContentLayout";
import SectionHeading from "@/components/headings/SectionHeading";
import Gateway from "@/components/icons/Gateway";
import { CurrentEpochCard } from "@/components/landingPageComponents/CurrentEpochCard";
import { NetworkStakeCard } from "@/components/landingPageComponents/NetworkStakeCard";
import { NoiseCard } from "@/components/landingPageComponents/NoiseCard";
import { RewardsCard } from "@/components/landingPageComponents/RewardsCard";
import { TokenomicsCard } from "@/components/landingPageComponents/TokenomicsCard";
import NodeTable from "@/components/nodeTable/NodeTableWithAction";
import NodeAndAddressSearch from "@/components/search/NodeAndAddressSearch";
import { Stack, Typography } from "@mui/material";
import Grid from "@mui/material/Grid2";
import { Suspense } from "react";

export default function Home() {
  return (
    <ContentLayout component="main">
      <Stack gap={5}>
        <Typography variant="h1" textTransform={"uppercase"}>
          Mixnet in your hands
        </Typography>
        <NodeAndAddressSearch />
      </Stack>
      <Grid container columnSpacing={5} rowSpacing={5}>
        <Grid size={12}>
          <SectionHeading title="Noise Generating Mixnet Overview" />
        </Grid>
        <Grid size={{ xs: 12, md: 3 }}>
          <Suspense fallback={<CardSkeleton />}>
            <NoiseCard />
          </Suspense>
        </Grid>
        <Grid container rowSpacing={3} size={{ xs: 12, md: 3 }}>
          <Suspense fallback={<CardSkeleton sx={{ width: "100%" }} />}>
            <Stack gap={5} width="100%">
              <RewardsCard />
              <CurrentEpochCard />
            </Stack>
          </Suspense>
        </Grid>
        <Grid size={{ xs: 12, md: 3 }}>
          <Suspense fallback={<CardSkeleton sx={{ height: "100%" }} />}>
            <NetworkStakeCard />
          </Suspense>
        </Grid>
        <Grid size={{ xs: 12, md: 3 }}>
          <Suspense fallback={<CardSkeleton sx={{ height: "100%" }} />}>
            <TokenomicsCard />
          </Suspense>
        </Grid>
        <Grid size={12}>
          <SectionHeading title="Nym Nodes" />
        </Grid>
        <Grid size={12}>
          <Suspense fallback={<CardSkeleton />}>
            <NodeTable />
          </Suspense>
        </Grid>
        <Grid size={12}>
          <SectionHeading title="Onboarding" />
        </Grid>
        <Grid size={6}>
          <ExplorerHeroCard
            label="Onboarding"
            title="How to select Nym vpn gateway?"
            description="Stake your tokens to well performing mix nodes, and earn a share of operator rewards!"
            image={<Gateway />}
            link={"/onboarding"}
            sx={{ width: "100%" }}
          />
        </Grid>
        <Grid size={6}>
          <ExplorerHeroCard
            label="Onboarding"
            title="How to select Nym vpn gateway?"
            description="Stake your tokens to well performing mix nodes, and earn a share of operator rewards!"
            image={<Gateway />}
            link={"/onboarding"}
            sx={{ width: "100%" }}
          />
        </Grid>
      </Grid>
    </ContentLayout>
  );
}