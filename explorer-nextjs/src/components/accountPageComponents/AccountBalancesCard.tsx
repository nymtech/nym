"use client";
import { fetchAccountBalance, fetchNymPrice } from "@/app/api";
import { Stack, Typography } from "@mui/material";
import { useQuery } from "@tanstack/react-query";
import type { IAccountBalancesInfo, IRewardDetails } from "../../app/api/types";
import { AccountBalancesTable } from "../cards/AccountBalancesTable";
import ExplorerCard from "../cards/ExplorerCard";

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

const getNymsFormated = (unyms: number) => {
  const balance = unyms / 1000000;
  return balance;
};
const getPriceInUSD = (unyms: number, usdPrice: number) => {
  const balanceInUSD = (unyms / 1000000) * usdPrice;
  const balanceFormated = Number(balanceInUSD.toFixed(2));
  return balanceFormated;
};

const getAllocation = (unyms: number, totalUnyms: number): number => {
  const allocationPercentage = (unyms * 100) / totalUnyms;
  return Number(allocationPercentage.toFixed(2));
};

const calculateStakingRewards = (
  accumulatedRewards: IRewardDetails[],
): number => {
  const totalRewards = accumulatedRewards.reduce((total, rewardDetail) => {
    return total + Number.parseFloat(rewardDetail.rewards.amount);
  }, 0);

  const result = getNymsFormated(totalRewards);

  return result;
};

export const AccountBalancesCard = (props: IAccountBalancesCardProps) => {
  const { address } = props;

  const {
    data: accountInfo,
    isLoading,
    isError,
  } = useQuery({
    queryKey: ["accountBalance", address],
    queryFn: () => fetchAccountBalance(address),
    enabled: !!address,
  });

  const {
    data: nymPrice,
    isLoading: isLoadingPrice,
    error: priceError,
  } = useQuery({
    queryKey: ["nymPrice"],
    queryFn: fetchNymPrice,
  });

  if (isLoading || isLoadingPrice) {
    return (
      <Stack direction="row" spacing={1}>
        <Typography variant="h5" fontWeight="light">
          Loading account balance...
        </Typography>
      </Stack>
    );
  }

  if (isError || priceError || !accountInfo || !nymPrice) {
    return (
      <Stack direction="row" spacing={1}>
        <Typography variant="h5" fontWeight="light">
          Failed to load account balance.
        </Typography>
      </Stack>
    );
  }

  const nymPriceData = nymPrice.quotes.USD.price;

  const totalBalanceUSD = getPriceInUSD(
    Number(accountInfo.total_value.amount),
    nymPriceData,
  );
  const spendableNYM = getNymsFormated(Number(accountInfo.balances[0].amount));
  const spendableUSD = getPriceInUSD(
    Number(accountInfo.balances[0].amount),
    nymPriceData,
  );
  const spendableAllocation = getAllocation(
    Number(accountInfo.balances[0].amount),
    Number(accountInfo.total_value.amount),
  );

  const delegationsNYM = getNymsFormated(
    Number(accountInfo.total_delegations.amount),
  );
  const delegationsUSD = getPriceInUSD(
    Number(accountInfo.total_delegations.amount),
    nymPriceData,
  );
  const delegationsAllocation = getAllocation(
    Number(accountInfo.total_delegations.amount),
    Number(accountInfo.total_value.amount),
  );

  const claimableNYM = getNymsFormated(
    Number(accountInfo.claimable_rewards.amount),
  );
  const claimableUSD = getPriceInUSD(
    Number(accountInfo.claimable_rewards.amount),
    nymPriceData,
  );
  const claimableAllocation = getAllocation(
    Number(accountInfo.claimable_rewards.amount),
    Number(accountInfo.total_value.amount),
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
      type: "Self bonded",
      allocation: 0,
      amount: 0,
      value: 0,
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
