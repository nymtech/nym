"use client";
import { fetchEpochRewards, fetchNoise, fetchNymPrice } from "@/app/api";
import { formatBigNum } from "@/utils/formatBigNumbers";
import { Box, Skeleton, Stack, Typography, useTheme } from "@mui/material";
import { useQuery } from "@tanstack/react-query";
import type { ExplorerData, NymTokenomics } from "../../app/api/types";
import ExplorerCard from "../cards/ExplorerCard";
import ExplorerListItem from "../list/ListItem";
import { TitlePrice } from "../price/TitlePrice";

export const TokenomicsCard = () => {
  const theme = useTheme();
  const isDarkMode = theme.palette.mode === "dark";
  const {
    data: nymPrice,
    isLoading,
    isError,
  } = useQuery({
    queryKey: ["nymPrice"],
    queryFn: fetchNymPrice,
    staleTime: 10 * 60 * 1000, // 10 minutes
    refetchOnWindowFocus: false, // Prevents unnecessary refetching
    refetchOnReconnect: false,
    refetchOnMount: false,
  });

  const {
    data: epochRewards,
    isLoading: isEpochLoading,
    isError: isEpochError,
  } = useQuery({
    queryKey: ["epochRewards"],
    queryFn: fetchEpochRewards,
    staleTime: 10 * 60 * 1000, // 10 minutes
    refetchOnWindowFocus: false, // Prevents unnecessary refetching
    refetchOnReconnect: false,
    refetchOnMount: false,
  });

  const {
    data: packetsAndStaking,
    isLoading: isStakingLoading,
    isError: isStakingError,
  } = useQuery({
    queryKey: ["noise"],
    queryFn: fetchNoise,
    staleTime: 10 * 60 * 1000, // 10 minutes
    refetchOnWindowFocus: false, // Prevents unnecessary refetching
    refetchOnReconnect: false,
    refetchOnMount: false,
  });

  if (isLoading || isEpochLoading || isStakingLoading) {
    return (
      <ExplorerCard label="Tokenomics overview">
        <Stack gap={1}>
          <Skeleton variant="text" />
          <Skeleton variant="text" height={238} />
        </Stack>
      </ExplorerCard>
    );
  }

  if (
    isStakingError ||
    isEpochError ||
    isError ||
    !nymPrice ||
    !epochRewards ||
    !packetsAndStaking
  ) {
    return (
      <ExplorerCard label="Tokenomics overview">
        <Typography
          variant="h5"
          sx={{ color: isDarkMode ? "base.white" : "pine.950" }}
        >
          Failed to load tokenomics overview.
        </Typography>
        <Skeleton variant="text" height={80} />
      </ExplorerCard>
    );
  }

  const nymPriceData: NymTokenomics = nymPrice;
  const nymPriceDataFormated = Number(nymPriceData.quotes.USD.price.toFixed(4));

  const titlePrice = {
    price: nymPriceDataFormated,
    // upDownLine: {
    //   percentage: 10,
    //   numberWentUp: true,
    // },
  };
  const marketCap = formatBigNum(nymPriceData.quotes.USD.market_cap);
  const volume24H = formatBigNum(nymPriceData.quotes.USD.volume_24h);

  const epochRewardsData: ExplorerData["currentEpochRewardsData"] =
    epochRewards;
  const packetsAndStakingData: ExplorerData["packetsAndStakingData"] =
    packetsAndStaking;

  function calculateTVL(
    epochRewards: ExplorerData["currentEpochRewardsData"],
    nymPriceData: NymTokenomics,
    packetsAndStaking: ExplorerData["packetsAndStakingData"]
  ): number {
    const lastTotalStake =
      packetsAndStaking[packetsAndStaking.length - 1]?.total_stake || 0;
    return (
      (Number.parseFloat(epochRewards.interval.reward_pool) / 1000000 +
        lastTotalStake / 1000000) *
      nymPriceData.quotes.USD.price
    );
  }
  const TVL = formatBigNum(
    calculateTVL(epochRewardsData, nymPrice, packetsAndStakingData)
  );

  const dataRows = [
    { key: "Market cap", value: `$ ${marketCap}` },
    { key: "24H VOL", value: `$ ${volume24H}` },
    { key: "TVL", value: `$ ${TVL}` },
  ];

  return (
    <ExplorerCard label="Tokenomics overview" sx={{ height: "100%" }}>
      <Stack gap={3} height="100%">
        <TitlePrice {...titlePrice} />
        <Box>
          {dataRows.map((row, i) => (
            <ExplorerListItem
              key={row.key}
              label={row.key}
              value={row.value}
              row={true}
              divider={i < dataRows.length - 1}
            />
          ))}
        </Box>
      </Stack>
    </ExplorerCard>
  );
};
