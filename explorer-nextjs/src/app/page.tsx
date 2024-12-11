"use client";

import {
  AccountStatsCard,
  type IAccountStatsCardProps,
} from "@/components/Cards/AccountStatsCard";
import ExplorerCard from "@/components/Cards/ExplorerCard";
import ExplorerHeroCard from "@/components/Cards/ExplorerHeroCard";
import ExplorerListItem from "@/components/List/ListItem";
import ProgressBar from "@/components/RatingMeter/RatingMeter";
import StarRarating from "@/components/StarRating/StarRating";
import TwoSidedSwitch from "@/components/TwoSidedButtonSwitch";
import CopyFile from "@/components/icons/CopyFile";
import Gateway from "@/components/icons/Gateway";
import { CurrentEpochCard } from "@/components/landingPageComponents/CurrentEpochCard";
import { NetworkStakeCard } from "@/components/landingPageComponents/NetworkStakeCard";
import { NoiseCard } from "@/components/landingPageComponents/NoiseCard";
import { RewardsCard } from "@/components/landingPageComponents/RewardsCard";
import { TokenomicsCard } from "@/components/landingPageComponents/TokenomicsCard";
import { Wrapper } from "@/components/wrapper";
import { Container, Grid2, IconButton, Stack, Typography } from "@mui/material";
import Grid from "@mui/material/Grid2";
import React, { useEffect, useState } from "react";
import { type ContentCardProps, MonoCard } from "../components/cards/MonoCard";
import { type ExplorerData, getCacheExplorerData } from "./api";

const explorerCard: ContentCardProps = {
  overTitle: "SINGLE",
  profileImage: {},
  title: "SINGLE",
  profileCountry: {
    countryCode: "NO",
    countryName: "Norway",
  },
  upDownLine: {
    percentage: 10,
    numberWentUp: true,
  },
  titlePrice: {
    price: 1.15,
    upDownLine: {
      percentage: 10,
      numberWentUp: true,
    },
  },
  dataRows: {
    rows: [
      { key: "Market cap", value: "$ 1000000" },
      { key: "24H VOL", value: "$ 1000000" },
    ],
  },
  graph: {
    data: [
      {
        date_utc: "2024-11-20",
        numericData: 10,
      },
      {
        date_utc: "2024-11-21",
        numericData: 12,
      },
      {
        date_utc: "2024-11-22",
        numericData: 9,
      },
      {
        date_utc: "2024-11-23",
        numericData: 11,
      },
    ],
    color: "#00CA33",
    label: "Label",
  },
  nymAddress: {
    address: "n1w7tfthyfkhh3au3mqpy294p4dk65dzal2h04su",
    title: "Nym address",
  },
  identityKey: {
    address: "n1w7tfthyfkhh3au3mqpy294p4dk65dzal2h04su",
    title: "Nym address",
  },
  qrCode: {
    url: "https://nymtech.net",
  },
  ratings: {
    ratings: [
      { title: "Rating-1", numberOfStars: 4 },
      { title: "Rating-2", numberOfStars: 2 },
      { title: "Rating-3", numberOfStars: 3 },
    ],
  },
  progressBar: {
    overTitle: "Current NGM epoch",
    start: "2024-12-08T12:26:19Z",
    showEpoch: true,
  },
  comments: true,
  paragraph: "Additional line",
  stakeButton: {
    label: "Stake on node",
    identityKey: "n1w7tfthyfkhh3au3mqpy294p4dk65dzal2h04su",
  },
};

const accountStatsCard: IAccountStatsCardProps = {
  overTitle: "Total value",
  priceTitle: 1990.0174,
  rows: [
    { type: "Spendable", allocation: 15.53, amount: 12800, value: 1200 },
    {
      type: "Delegated",
      allocation: 15.53,
      amount: 12800,
      value: 1200,
      history: [
        { type: "Liquid", amount: 6900 },
        { type: "Locked", amount: 6900 },
      ],
    },
    {
      type: "Claimable",
      allocation: 15.53,
      amount: 12800,
      value: 1200,
      history: [
        { type: "Unlocked", amount: 6900 },
        { type: "Staking rewards", amount: 6900 },
        { type: "Operator comission", amount: 6900 },
      ],
    },
    {
      type: "Self bonded",
      allocation: 15.53,
      amount: 12800,
      value: 1200,
    },
    {
      type: "Locked",
      allocation: 15.53,
      amount: 12800,
      value: 1200,
    },
  ],
};

export default function Home() {
  const [explorerData, setExplorerData] = useState<ExplorerData>();

  useEffect(() => {
    async function fetchData() {
      const data = await getCacheExplorerData();
      setExplorerData(data);
    }
    fetchData();
  }, []);

  return (
    <div>
      <main>
        <Wrapper>
          <Container maxWidth="lg">
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
              <Typography variant="h1" textTransform={"uppercase"} mb={5}>
                Mixnet in your hands
              </Typography>
              <Grid
                container
                rowSpacing={3}
                columnSpacing={2}
                mb={2}
                alignItems="stretch"
              >
                <Grid size={{ xs: 12, md: 3 }}>
                  {explorerData && <NoiseCard explorerData={explorerData} />}
                </Grid>
                <Grid container rowSpacing={3} size={{ xs: 12, md: 3 }}>
                  <Grid size={{ xs: 12 }}>
                    <RewardsCard />
                  </Grid>
                  <Grid size={{ xs: 12 }}>
                    {explorerData && (
                      <CurrentEpochCard explorerData={explorerData} />
                    )}
                  </Grid>
                </Grid>
                <Grid size={{ xs: 12, md: 3 }} height={"100%"}>
                  {explorerData && (
                    <NetworkStakeCard explorerData={explorerData} />
                  )}
                </Grid>
                <Grid size={{ xs: 12, md: 3 }}>
                  <TokenomicsCard />
                </Grid>
              </Grid>
              <Grid container gap={2} alignItems={"flex-start"}>
                <Grid size={{ xs: 12, md: 5 }}>
                  <MonoCard {...explorerCard} />
                </Grid>
                <Grid container size={{ xs: 6 }}>
                  <Grid size={{ xs: 12 }}>
                    <TwoSidedSwitch
                      leftLabel="Account"
                      rightLabel="Mixnode"
                      // onSwitch={() => console.log("object :>> ")}
                    />
                  </Grid>
                  <Grid size={{ xs: 12 }}>
                    <AccountStatsCard {...accountStatsCard} />
                  </Grid>
                </Grid>
              </Grid>
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
