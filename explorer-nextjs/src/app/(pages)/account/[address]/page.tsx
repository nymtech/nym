import type { CurrencyRates, IAccountBalancesInfo } from "@/app/api/types";
import type NodeData from "@/app/api/types";
import { NYM_ACCOUNT_ADDRESS, NYM_NODES, NYM_PRICES_API } from "@/app/api/urls";
import { AccountBalancesCard } from "@/components/accountPageComponents/AccountBalancesCard";
import { AccountInfoCard } from "@/components/accountPageComponents/AccountInfoCard";
import { ContentLayout } from "@/components/contentLayout/ContentLayout";
import SectionHeading from "@/components/headings/SectionHeading";
import ExplorerButtonGroup from "@/components/toggleButton/ToggleButton";
import { Box, Grid2, Typography } from "@mui/material";

export default async function Account({
  params,
}: {
  params: Promise<{ address: string }>;
}) {
  try {
    const { address } = await params;

    const accountData = await fetch(`${NYM_ACCOUNT_ADDRESS}${address}`, {
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json; charset=utf-8",
      },
      next: { revalidate: 60 },
      // refresh event list cache at given interval
    });

    const response = await fetch(NYM_NODES, {
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json; charset=utf-8",
      },
      next: { revalidate: 60 },
      // refresh event list cache at given interval
    });

    const nymNodes: NodeData[] = await response.json();

    const nymNode = nymNodes.find(
      (node) => node.bond_information.owner === address,
    );

    const nymAccountBalancesData: IAccountBalancesInfo =
      await accountData.json();

    if (!nymAccountBalancesData) {
      return <Typography>Account not found</Typography>;
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
                  {
                    label: "Nym Node",
                    isSelected: false,
                    link: nymNode
                      ? `/nym-node/${nymNode.node_id}`
                      : "/nym-node/not-found",
                  },
                  {
                    label: "Account",
                    isSelected: true,
                    link: `/account/${address}`,
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
  } catch (error) {
    console.error("error :>> ", error);
    return <Typography>Error loading account data</Typography>;
  }
}
