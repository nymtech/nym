"use client";
import type { IAccountInfo } from "@/app/account/[id]/page";
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
  accountInfo: IAccountInfo;
  nymPrice: number;
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

export const AccountBalancesCard = (props: IAccountBalancesCardProps) => {
  const { accountInfo, nymPrice } = props;

  const totalBalanceUSD = getPriceInUSD(
    Number(accountInfo.total_value.amount),
    nymPrice,
  );

  const spendableNYM = getNymsFormated(Number(accountInfo.balances[0].amount));
  const spendableUSD = getPriceInUSD(
    Number(accountInfo.balances[0].amount),
    nymPrice,
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
    nymPrice,
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
    nymPrice,
  );
  const claimableAllocation = getAllocation(
    Number(accountInfo.claimable_rewards.amount),
    Number(accountInfo.total_value.amount),
  );

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
      history: [
        { type: "Liquid", amount: 6900 },
        { type: "Locked", amount: 6900 },
      ],
    },
    {
      type: "Claimable",
      allocation: claimableAllocation,
      amount: claimableNYM,
      value: claimableUSD,
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
