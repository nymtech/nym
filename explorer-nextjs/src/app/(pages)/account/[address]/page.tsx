import { Box, Typography } from "@mui/material";
import Grid from "@mui/material/Grid2";
import { AccountBalancesCard } from "../../../../components/accountPageComponents/AccountBalancesCard";
import { AccountInfoCard } from "../../../../components/accountPageComponents/AccountInfoCard";
import BlogArticlesCards from "../../../../components/blogs/BlogArticleCards";
import { ContentLayout } from "../../../../components/contentLayout/ContentLayout";
import SectionHeading from "../../../../components/headings/SectionHeading";
import ExplorerButtonGroup from "../../../../components/toggleButton/ToggleButton";
import type { IAccountBalancesInfo, NymTokenomics } from "../../../api/types";
import type NodeData from "../../../api/types";
import {
  NYM_ACCOUNT_ADDRESS,
  NYM_NODES,
  NYM_PRICES_API,
} from "../../../api/urls";

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

    const nymPriceData: NymTokenomics = await nymPrice.json();

    return (
      <ContentLayout>
        <Grid container columnSpacing={5} rowSpacing={5}>
          <Grid size={6}>
            <SectionHeading title="Account Details" />
          </Grid>
          <Grid size={6} justifyContent="flex-end">
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
          </Grid>
          <Grid size={4}>
            <AccountInfoCard accountInfo={nymAccountBalancesData} />
          </Grid>
          <Grid size={8}>
            <AccountBalancesCard
              accountInfo={nymAccountBalancesData}
              nymPrice={nymPriceData.quotes.USD.price}
            />
          </Grid>
        </Grid>
        <Grid container columnSpacing={5} rowSpacing={5}>
          <Grid size={12}>
            <SectionHeading title="Onboarding" />
          </Grid>
          <BlogArticlesCards limit={4} />
        </Grid>
      </ContentLayout>
    );
  } catch (error) {
    console.error("error :>> ", error);
    return <Typography>Error loading account data</Typography>;
  }
}
