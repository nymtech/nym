import type { CurrencyRates } from "@/app/api/types";
import { NYM_ACCOUNT_ADDRESS, NYM_PRICES_API } from "@/app/api/urls";
import { AccountBalancesCard } from "@/components/accountPageComponents/AccountBalancesCard";
import { AccountInfoCard } from "@/components/accountPageComponents/AccountInfoCard";
import { ContentLayout } from "@/components/contentLayout/ContentLayout";
import SectionHeading from "@/components/headings/SectionHeading";
import ExplorerButtonGroup from "@/components/toggleButton/ToggleButton";
import { Box, Grid2 } from "@mui/material";

interface IRewardDetails {
  amount_staked: IAmountDetails;
  node_id: number;
  node_still_fully_bonded: boolean;
  rewards: IAmountDetails;
}

interface IAmountDetails {
  denom: string;
  amount: string;
}

interface IDelegationDetails {
  node_id: number;
  delegated: IAmountDetails;
  height: number;
  proxy: null | string;
}

interface ITotalDetails {
  amount: string;
  denom: string;
}

export interface IAccountInfo {
  accumulated_rewards: IRewardDetails[];
  address: string;
  balances: IAmountDetails[];
  claimable_rewards: IAmountDetails;
  delegations: IDelegationDetails[];
  operator_rewards: null | any;
  total_delegations: ITotalDetails;
  total_value: ITotalDetails;
  vesting_account: null | any;
}

export default async function Account({
  params,
}: {
  params: Promise<{ address: string }>;
}) {
  // const address = (await params).address;
  console.log("(await params).address :>> ", (await params).address);
  const address = "n1z0msxu8c098umdhnthpr2ac3ck2n3an97dm8pn";

  const nymAccountAddress = `${NYM_ACCOUNT_ADDRESS}${address}`;
  const accountData = await fetch(nymAccountAddress, {
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json; charset=utf-8",
    },
    next: { revalidate: 60 },
    // refresh event list cache at given interval
  });
  const nymAccountData: IAccountInfo = await accountData.json();

  if (!nymAccountData) {
    return null;
  }

  const nymPrice = await fetch(NYM_PRICES_API, {
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json; charset=utf-8",
    },
    next: { revalidate: 60 },
    // refresh event list cache at given interval
  });

  const nymPriceData: CurrencyRates = await nymPrice.json();

  console.log("nymPriceData :>> ", nymPriceData);

  console.log("nymAccountData :>> ", nymAccountData);
  return (
    <ContentLayout>
      <Grid2 container columnSpacing={5} rowSpacing={5}>
        <Grid2 size={6}>
          <SectionHeading title="Account Details" />
        </Grid2>
        <Grid2 size={6} justifyContent="flex-end">
          <Box sx={{ display: "flex", justifyContent: "end" }}>
            <ExplorerButtonGroup
              options={[
                { label: "Nym Node", isSelected: false, link: "/nym-node/1" },
                {
                  label: "Account",
                  isSelected: true,
                  link: "/account/1",
                },
              ]}
            />
          </Box>
        </Grid2>
        <Grid2 size={4}>
          <AccountInfoCard accountInfo={nymAccountData} />
        </Grid2>
        <Grid2 size={8}>
          <AccountBalancesCard
            accountInfo={nymAccountData}
            nymPrice={nymPriceData.usd}
          />
        </Grid2>
      </Grid2>
    </ContentLayout>
  );
}
