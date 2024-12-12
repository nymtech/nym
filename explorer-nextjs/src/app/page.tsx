import ExplorerHeroCard from "@/components/cards/ExplorerHeroCard";
import Gateway from "@/components/icons/Gateway";
import { NetworkStakeCard } from "@/components/landingPageComponents/NetworkStakeCard";
import { NoiseCard } from "@/components/landingPageComponents/NoiseCard";
import { RewardsCard } from "@/components/landingPageComponents/RewardsCard";
import { TokenomicsCard } from "@/components/landingPageComponents/TokenomicsCard";
import { Wrapper } from "@/components/wrapper";
import { Box, Stack, Typography } from "@mui/material";
import Grid from "@mui/material/Grid2";

export default function Home() {
  return (
    <div>
      <main>
        <Box sx={{ p: 10 }}>
          <Wrapper>
            <Typography variant="h1" textTransform={"uppercase"} mb={10}>
              Mixnet in your hands
            </Typography>
            <Grid container columnSpacing={5} rowSpacing={5}>
              <Grid size={12}>
                <Typography variant="h2">
                  Noise Generating Mixnet Overview
                </Typography>
              </Grid>
              <Grid size={{ xs: 12, md: 3 }}>
                <NoiseCard />
              </Grid>
              <Grid container rowSpacing={3} size={{ xs: 12, md: 3 }}>
                <Stack gap={5}>
                  <RewardsCard />
                  {/* <CurrentEpochCard /> */}
                </Stack>
              </Grid>
              <Grid size={{ xs: 12, md: 3 }}>
                <NetworkStakeCard />
              </Grid>
              <Grid size={{ xs: 12, md: 3 }}>
                <TokenomicsCard />
              </Grid>
              <Grid size={12}>
                <Typography variant="h2">Onboarding</Typography>
              </Grid>
              <Grid size={6}>
                <ExplorerHeroCard
                  label="Onboarding"
                  title="How to select Nym vpn gateway?"
                  description="Stake your tokens to well performing mix nodes, and earn a share of operator rewards!"
                  image={<Gateway />}
                  link={"/onboarding"}
                />
              </Grid>
              <Grid size={6}>
                <ExplorerHeroCard
                  label="Onboarding"
                  title="How to select Nym vpn gateway?"
                  description="Stake your tokens to well performing mix nodes, and earn a share of operator rewards!"
                  image={<Gateway />}
                  link={"/onboarding"}
                />
              </Grid>
            </Grid>
          </Wrapper>
        </Box>
      </main>
    </div>
  );
}
