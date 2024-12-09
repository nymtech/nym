import CopyToClipboard from "@/components/copyToClipboard/CopyToClipboard";
import Gateway from "@/components/icons/Gateway";
import ExplorerButtonGroup from "@/components/toggleButton/ToggleButton";
"use client";
import {
  AccountStatsCard,
  type IAccountStatsCardProps,
} from "@/components/cards/AccountStatsCard";
import TwoSidedSwitch from "@/components/twoSidedSwitchButton";
import { Wrapper } from "@/components/wrapper";
import { Box, Typography } from "@mui/material";
import Grid from "@mui/material/Grid2";
import React from "react";
import { type ContentCardProps, MonoCard } from "../components/cards/MonoCard";

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
    title: "Current NGM epoch",
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
  return (
    <div>
      <main>
        <Box sx={{ p: 5 }}>
          <Wrapper>
            <Typography fontWeight="light">
              🚀 EXPLORER 2.0, Let&apos;s go! 🚀
            </Typography>
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
          </Wrapper>
        </Box>
      </main>
    </div>
  );
}
