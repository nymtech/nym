"use client";
import { fetchAccountBalance, fetchNymPrice } from "@/app/api";
import { Skeleton, Stack, Typography, useTheme } from "@mui/material";
import { useQuery } from "@tanstack/react-query";
import type { IRewardDetails } from "../../app/api/types";
import ExplorerCard from "../cards/ExplorerCard";
import { AccountBalancesTable } from "./AccountBalancesTable";
import { useEnvironment } from "@/providers/EnvironmentProvider";

export interface IAccontStatsRowProps {
  type: string;
  allocation: number;
  amount: number;
  value: number;
  history?: { type: string; amount: number }[];
  isLastRow?: boolean;
  progressBarColor?: string;
}

interface IAccountBalancesCardProps {
  address: string;
}

const getNymsFormated = (unyms: number): number => {
  if (unyms === 0) {
    return 0;
  }
  const balance = unyms / 1000000;
  return balance;
};
const getPriceInUSD = (unyms: number, usdPrice: number): number => {
  if (unyms === 0) {
    return 0;
  }
  const balanceInUSD = (unyms / 1000000) * usdPrice;
  const balanceFormated = Number(balanceInUSD.toFixed(2));
  return balanceFormated;
};

const getAllocation = (unyms: number, totalUnyms: number): number => {
  if (unyms === 0) {
    return 0;
  }
  const allocationPercentage = (unyms * 100) / totalUnyms;
  return Number(allocationPercentage.toFixed(2));
};

const calculateStakingRewards = (
  accumulatedRewards: IRewardDetails[]
): number => {
  if (accumulatedRewards.length > 0) {
    const totalRewards = accumulatedRewards.reduce((total, rewardDetail) => {
      return total + Number.parseFloat(rewardDetail.rewards.amount);
    }, 0);

    const result = getNymsFormated(totalRewards);

    return result;
  }
  return 0;
};

export const AccountBalancesCard = (props: IAccountBalancesCardProps) => {
  const { address } = props;
  const theme = useTheme();
  const isDarkMode = theme.palette.mode === "dark";
  const { environment } = useEnvironment();

  const {
    data: accountInfo,
    isLoading,
    isError,
  } = useQuery({
    queryKey: ["accountBalance", address, environment],
    queryFn: () => fetchAccountBalance(address, environment),
    enabled: !!address,
    staleTime: 10 * 60 * 1000, // 10 minutes
    refetchOnWindowFocus: false, // Prevents unnecessary refetching
    refetchOnReconnect: false,
  });

  const {
    data: nymPrice,
    isLoading: isNymPriceLoading,
    isError: isNymPriceError,
  } = useQuery({
    queryKey: ["nymPrice"],
    queryFn: fetchNymPrice,
    staleTime: 10 * 60 * 1000, // 10 minutes
    refetchOnWindowFocus: false, // Prevents unnecessary refetching
    refetchOnReconnect: false,
    refetchOnMount: false,
  });

  if (isLoading || isNymPriceLoading) {
    return (
      <ExplorerCard label="Total value">
        <Stack gap={1}>
          <Skeleton variant="text" height={38} />
          <Skeleton variant="text" height={380} />
        </Stack>
      </ExplorerCard>
    );
  }

  if (isError || isNymPriceError || !accountInfo || !nymPrice) {
    return (
      <ExplorerCard label="Total value">
        <Typography
          variant="h5"
          sx={{ color: isDarkMode ? "base.white" : "pine.950" }}
        >
          Failed to account data.
        </Typography>
        <Skeleton variant="text" height={238} />
      </ExplorerCard>
    );
  }

  const nymPriceData = nymPrice.quotes.USD.price;

  const totalBalanceUSD = getPriceInUSD(
    Number(accountInfo.total_value.amount),
    nymPriceData
  );
  const spendableNYM = accountInfo.balance
    ? getNymsFormated(Number(accountInfo.balance.amount))
    : 0;
  const spendableUSD = accountInfo.balance
    ? getPriceInUSD(Number(accountInfo.balance.amount), nymPriceData)
    : 0;
  const spendableAllocation = accountInfo.balance
    ? getAllocation(
        Number(accountInfo.balance.amount),
        Number(accountInfo.total_value.amount)
      )
    : 0;

  const delegationsNYM = getNymsFormated(
    Number(accountInfo.total_delegations.amount)
  );
  const delegationsUSD = getPriceInUSD(
    Number(accountInfo.total_delegations.amount),
    nymPriceData
  );
  const delegationsAllocation = getAllocation(
    Number(accountInfo.total_delegations.amount),
    Number(accountInfo.total_value.amount)
  );

  const operatorRewardsAllocation = getAllocation(
    Number(accountInfo.operator_rewards?.amount || 0),
    Number(accountInfo.total_value.amount)
  );

  const operatorRewardsNYM = getNymsFormated(
    Number(accountInfo.operator_rewards?.amount || 0)
  );

  const operatorRewardsUSD = getPriceInUSD(
    Number(accountInfo.operator_rewards?.amount || 0),
    nymPriceData
  );

  const claimableNYM = getNymsFormated(
    Number(accountInfo.claimable_rewards.amount)
  );
  const claimableUSD = getPriceInUSD(
    Number(accountInfo.claimable_rewards.amount),
    nymPriceData
  );
  const claimableAllocation = getAllocation(
    Number(accountInfo.claimable_rewards.amount),
    Number(accountInfo.total_value.amount)
  );

  const stakingRewards =
    accountInfo.accumulated_rewards.length > 0
      ? calculateStakingRewards(accountInfo.accumulated_rewards)
      : 0;

  const tableRows = [
    {
      type: "Spendable",
      allocation: spendableAllocation,
      amount: spendableNYM,
      value: spendableUSD,
    },
    {
      type: "Delegated",
      allocation: delegationsAllocation,
      amount: delegationsNYM,
      value: delegationsUSD,
      // history: [
      //   { type: "Liquid", amount: 6900 },
      //   { type: "Locked", amount: 6900 },
      // ],
    },
    {
      type: "Claimable",
      allocation: claimableAllocation,
      amount: claimableNYM,
      value: claimableUSD,
      history: [
        // { type: "Unlocked", amount: 6900 },
        {
          type: "Staking rewards",
          amount: stakingRewards,
        },
        { type: "Operator comission", amount: 0 },
      ],
    },
    {
      type: "Operator Rewards",
      allocation: operatorRewardsAllocation,
      amount: operatorRewardsNYM,
      value: operatorRewardsUSD,
    },
  ];

  return (
    <ExplorerCard
      label="Total value"
      title={`$ ${totalBalanceUSD}`}
      sx={{ height: "100%" }}
    >
      <AccountBalancesTable rows={tableRows} />
    </ExplorerCard>
  );
};
