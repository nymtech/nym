import type { IBondInfo, INodeDescription } from "@/app/api";
import { NYM_NODE_BONDED, NYM_NODE_DESCRIPTION } from "@/app/api/urls";
import ExplorerCard from "@/components/cards/ExplorerCard";
import { ContentLayout } from "@/components/contentLayout/ContentLayout";
import SectionHeading from "@/components/headings/SectionHeading";
import ExplorerListItem from "@/components/list/ListItem";
import { BasicInfoCard } from "@/components/nymNodePageComponents/BasicInfoCard";
import { NodeMetricsCard } from "@/components/nymNodePageComponents/NodeMetricsCard";
import { StarRating } from "@/components/starRating";
import ExplorerButtonGroup from "@/components/toggleButton/ToggleButton";
import { Box, Grid2, Stack } from "@mui/material";

export default async function NymNode({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const id = (await params).id;

  const descriptionData = await fetch(NYM_NODE_DESCRIPTION, {
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json; charset=utf-8",
    },
    next: { revalidate: 60 },
    // refresh event list cache at given interval
  });
  const nymNodesDescription = await descriptionData.json();

  if (!nymNodesDescription) {
    return null;
  }

  console.log("id :>> ", id);

  const bondedData = await fetch(NYM_NODE_BONDED, {
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json; charset=utf-8",
    },
    next: { revalidate: 60 },
    // refresh event list cache at given interval
  });
  const nymbondedData = await bondedData.json();

  if (!bondedData) {
    return null;
  }

  const nodeBondInfo = nymbondedData.data.filter(
    (item: IBondInfo) => item.bond_information.node_id === 5,
  );

  const nodeDescriptionInfo = nymNodesDescription.data.filter(
    (item: INodeDescription) => item.node_id === 5,
  );

  console.log("nodeDescriptionInfo :>> ", nodeDescriptionInfo);

  return (
    <ContentLayout>
      <Grid2 container columnSpacing={5} rowSpacing={5}>
        <Grid2 size={6}>
          <SectionHeading title="Nym Node Details" />
        </Grid2>
        <Grid2 size={6} justifyContent="flex-end">
          <Box sx={{ display: "flex", justifyContent: "end" }}>
            <ExplorerButtonGroup
              options={[
                { label: "Nym Node", isSelected: true, link: "/nym-node/1" },
                {
                  label: "Account",
                  isSelected: false,
                  link: "/account/1",
                },
              ]}
            />
          </Box>
        </Grid2>
        <Grid2 size={4}>
          <ExplorerCard label="Action" sx={{ height: "100%" }}>
            <div />
          </ExplorerCard>
        </Grid2>
        <Grid2 size={4}>
          <BasicInfoCard
            bondInfo={nodeBondInfo[0]}
            nodeDescription={nodeDescriptionInfo[0]}
          />
        </Grid2>
        <Grid2 size={4}>
          <ExplorerCard
            label="Node Rewards (Last Epoch/Hour)"
            sx={{ height: "100%" }}
          >
            <ExplorerListItem row divider label="Role" value="Gateway" />
            <ExplorerListItem
              row
              divider
              label="Quality of service"
              value={<StarRating value={5} />}
            />
            <ExplorerListItem
              row
              divider
              label="Config score"
              value={<StarRating value={4} />}
            />
            <ExplorerListItem
              row
              divider
              label="Probe score"
              value={<StarRating value={5} />}
            />
          </ExplorerCard>
        </Grid2>
        <Grid2 size={6}>
          <ExplorerCard label="Nym node metrics" sx={{ height: "100%" }}>
            <ExplorerListItem
              row
              divider
              label="Total rew."
              value="10,000 NYM"
            />
            <ExplorerListItem
              row
              divider
              label="Operator rew."
              value="10,000 NYM"
            />
            <ExplorerListItem
              row
              divider
              label="Staker rew."
              value="10,000 NYM"
            />
            <ExplorerListItem
              row
              divider
              label="Profit margin rew."
              value="40 NYM"
            />
            <ExplorerListItem
              row
              divider
              label="Operating cost."
              value="40 NYM"
            />
          </ExplorerCard>
        </Grid2>
        <Grid2 size={6}>
          <NodeMetricsCard nodeDescription={nodeDescriptionInfo[0]} />
        </Grid2>
      </Grid2>
    </ContentLayout>
  );
}
