import ExplorerCard from "@/components/Cards/ExplorerCard";
import ExplorerHeroCard from "@/components/Cards/ExplorerHeroCard";
import ExplorerListItem from "@/components/List/ListItem";
import ProgressBar from "@/components/RatingMeter/RatingMeter";
import StarRarating from "@/components/StarRating/StarRating";
import CopyFile from "@/components/icons/CopyFile";
import Gateway from "@/components/icons/Gateway";
import { Wrapper } from "@/components/wrapper";
import { Container, Grid2, IconButton, Stack, Typography } from "@mui/material";

export default function Home() {
  return (
    <div>
      <main>
        <Wrapper>
          <Container maxWidth="md">
            <Stack spacing={4}>
              <ExplorerCard title="Explorer Card" subtitle="Cryptosailors">
                <ExplorerListItem
                  label="Identity Key"
                  value="n1w7tfthyfkhh3au3mqpy294p4dk65dzal2h04su"
                />
                <ExplorerListItem
                  label="Nym Address"
                  value={
                    <Stack direction="row" gap={0.1} alignItems="center">
                      <Typography variant="body4">
                        n1w7tfthyfkhh3au3mqpy294p4dk65dzal2h04su
                      </Typography>
                      <IconButton size="small">
                        <CopyFile />
                      </IconButton>
                    </Stack>
                  }
                />
                <ExplorerListItem
                  label="Star Rating"
                  value={<StarRarating value={3} />}
                />
                <ExplorerListItem
                  label="Progress bar"
                  value={<ProgressBar value={50} color="secondary" />}
                />
              </ExplorerCard>
              <Grid2 container spacing={4}>
                <Grid2 size={6}>
                  <ExplorerHeroCard
                    label="Onboarding"
                    title="How to select Nym vpn gateway?"
                    description="Stake your tokens to well performing mix nodes, and earn a share of operator rewards!"
                    image={<Gateway />}
                    link={"/onboarding"}
                  />
                </Grid2>
                <Grid2 size={6}>
                  <ExplorerHeroCard
                    label="Onboarding"
                    title="How to select Nym vpn gateway?"
                    description="Stake your tokens to well performing mix nodes, and earn a share of operator rewards!"
                    image={<Gateway />}
                    link={"/onboarding"}
                  />
                </Grid2>
              </Grid2>
            </Stack>
          </Container>
        </Wrapper>
      </main>
    </div>
  );
}
