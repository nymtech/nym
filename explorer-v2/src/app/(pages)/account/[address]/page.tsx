import { fetchObservatoryNodes } from "@/app/api";
import type { IObservatoryNode } from "@/app/api/types";
import { Box, Typography } from "@mui/material";
import Grid from "@mui/material/Grid2";
import { AccountBalancesCard } from "../../../../components/accountPageComponents/AccountBalancesCard";
import { AccountInfoCard } from "../../../../components/accountPageComponents/AccountInfoCard";
import { ContentLayout } from "../../../../components/contentLayout/ContentLayout";
import SectionHeading from "../../../../components/headings/SectionHeading";
import ExplorerButtonGroup from "../../../../components/toggleButton/ToggleButton";

export default async function Account({
  params,
}: {
  params: Promise<{ address: string }>;
}) {
  try {
    const address = (await params).address;

    const nymNodes: IObservatoryNode[] = await fetchObservatoryNodes();

    const nymNode = nymNodes.find(
      (node) => node.bonding_address === address,
    );

    return (
      <ContentLayout>
        <Grid container columnSpacing={5} rowSpacing={5}>
          <Grid size={6}>
            <SectionHeading title="Account Details" />
          </Grid>

          <Grid size={6} justifyContent="flex-end">
            <Box sx={{ display: "flex", justifyContent: "end" }}>
              <ExplorerButtonGroup
                onPage="Account"
                options={[
                  {
                    label: "Nym Node",
                    isSelected: false,
                    link: nymNode
                      ? `/nym-node/${nymNode.node_id}`
                      : `/account/${address}/not-found`,
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

          <Grid size={{ xs: 12, md: 4 }}>
            <AccountInfoCard address={address} />
          </Grid>
          <Grid size={{ xs: 12, md: 8 }}>
            <AccountBalancesCard address={address} />
          </Grid>
        </Grid>
      </ContentLayout>
    );
  } catch (error) {
    console.error("error :>> ", error);
    return <Typography>Error loading account data</Typography>;
  }
}
