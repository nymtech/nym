import type { CurrencyRates, IAccountBalancesInfo } from "@/app/api/types";
import { NYM_ACCOUNT_ADDRESS, NYM_PRICES_API } from "@/app/api/urls";
import { AccountBalancesCard } from "@/components/accountPageComponents/AccountBalancesCard";
import { AccountInfoCard } from "@/components/accountPageComponents/AccountInfoCard";
import { ContentLayout } from "@/components/contentLayout/ContentLayout";
import SectionHeading from "@/components/headings/SectionHeading";
import ExplorerButtonGroup from "@/components/toggleButton/ToggleButton";
import { Box, Grid2 } from "@mui/material";

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
  const nymAccountBalancesData: IAccountBalancesInfo = await accountData.json();

  if (!nymAccountBalancesData) {
    return null;
  }
  console.log("nymAccountBalancesData :>> ", nymAccountBalancesData);
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

  console.log("nymAccountData :>> ", nymAccountBalancesData);
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
          <AccountInfoCard accountInfo={nymAccountBalancesData} />
        </Grid2>
        <Grid2 size={8}>
          <AccountBalancesCard
            accountInfo={nymAccountBalancesData}
            nymPrice={nymPriceData.usd}
          />
        </Grid2>
      </Grid2>
    </ContentLayout>
  );
}
