import type { IBondInfo, INodeDescription } from "@/app/api";
import { NYM_NODE_BONDED, NYM_NODE_DESCRIPTION } from "@/app/api/urls";
import ExplorerCard from "@/components/cards/ExplorerCard";
import { ContentLayout } from "@/components/contentLayout/ContentLayout";
import SectionHeading from "@/components/headings/SectionHeading";
import ExplorerListItem from "@/components/list/ListItem";
import { BasicInfoCard } from "@/components/nymNodePageComponents/BasicInfoCard";
import { NodeMetricsCard } from "@/components/nymNodePageComponents/NodeMetricsCard";
import { NodeRewardsCard } from "@/components/nymNodePageComponents/NodeRewardsCard";
import { StarRating } from "@/components/starRating";
import ExplorerButtonGroup from "@/components/toggleButton/ToggleButton";
import { Box, Grid2 } from "@mui/material";

export default async function NymNode({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const id = Number((await params).id);

  const descriptionData = await fetch(NYM_NODE_DESCRIPTION, {
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json; charset=utf-8",
    },
    next: { revalidate: 60 },
    // refresh event list cache at given interval
  });
  const nymNodesDescription = await descriptionData.json();

  const bondedData = await fetch(NYM_NODE_BONDED, {
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json; charset=utf-8",
    },
    next: { revalidate: 60 },
    // refresh event list cache at given interval
  });
  const nymbondedData = await bondedData.json();

  if (!bondedData || !nymNodesDescription) {
    return null;
  }

  const nodeBondInfo = nymbondedData.data.filter(
    (item: IBondInfo) => item.bond_information.node_id === id,
  );

  const nodeDescriptionInfo = nymNodesDescription.data.filter(
    (item: INodeDescription) => item.node_id === 5,
  );

  return (
    <ContentLayout>
      <Grid2 container columnSpacing={5} rowSpacing={5}>
        <Grid2 size={12}>
          <Box sx={{ display: "flex", justifyContent: "space-between" }}>
            <SectionHeading title="Nym Node Details" />
            <ExplorerButtonGroup
              options={[
                { label: "Nym Node", isSelected: true, link: "/nym-node/1" },
                {
                  label: "Account",
                  isSelected: false,
                  link: `/account/${nymNode.bond_information.owner}`,
                },
              ]}
            />
          </Box>
        </Grid2>
        <Grid2
          size={{
            xs: 12,
            md: 4,
          }}
        >
          <NodeProfileCard
            bondInfo={nymNode.bond_information}
            nodeDescription={nymNode.description}
          />
        </Grid2>
        <Grid2 size={4}>
          <BasicInfoCard
            bondInfo={nodeBondInfo[0]}
            nodeDescription={nodeDescriptionInfo[0]}
          />
        </Grid2>
        <Grid2 size={4}>
          <ExplorerCard label="Quality indicatiors" sx={{ height: "100%" }}>
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
          <NodeRewardsCard bondInfo={nodeBondInfo[0]} />
        </Grid2>
        <Grid2 size={6}>
          <NodeMetricsCard nodeDescription={nodeDescriptionInfo[0]} />
        </Grid2>
      </Grid2>
    </ContentLayout>
  );
}
