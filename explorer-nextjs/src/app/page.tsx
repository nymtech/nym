import ExplorerCard from "@/components/cards/ExplorerCard";
import ExplorerHeroCard from "@/components/cards/ExplorerHeroCard";
import CopyToClipboard from "@/components/copyToClipboard/CopyToClipboard";
import Gateway from "@/components/icons/Gateway";
import ExplorerListItem from "@/components/list/ListItem";
import ProgressBar from "@/components/progressBar/ProgressBar";
import StarRarating from "@/components/starRating/StarRating";
import ExplorerButtonGroup from "@/components/toggleButton/ToggleButton";
import { Wrapper } from "@/components/wrapper";
import { Container, Grid2, Stack, Typography } from "@mui/material";

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
                    <Stack direction="row" gap={0.5} alignItems="center">
                      <Typography variant="body4">
                        n1w7tfthyfkhh3au3mqpy294p4dk65dzal2h04su
                      </Typography>
                      <CopyToClipboard text="n1w7tfthyfkhh3au3mqpy294p4dk65dzal2h04su" />
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
                <ExplorerListItem
                  label="Button group"
                  value={
                    <ExplorerButtonGroup
                      options={[
                        {
                          label: "Node",
                          link: "/node",
                          isSelected: true,
                        },
                        {
                          label: "Account",
                          link: "/account",
                          isSelected: false,
                        },
                      ]}
                    />
                  }
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
