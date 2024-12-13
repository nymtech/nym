import type { IBondInfo, INodeDescription } from "@/app/api";
import { NYM_NODE_BONDED, NYM_NODE_DESCRIPTION } from "@/app/api/urls";
import ExplorerCard from "@/components/cards/ExplorerCard";
import { ContentLayout } from "@/components/contentLayout/ContentLayout";
import SectionHeading from "@/components/headings/SectionHeading";
import ExplorerListItem from "@/components/list/ListItem";
import { BasicInfoCard } from "@/components/nymNodePageComponents/BasicInfoCard";
import { StarRating } from "@/components/starRating";
import ExplorerButtonGroup from "@/components/toggleButton/ToggleButton";
import { Box, Grid2 } from "@mui/material";

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
        <Grid2
          size={{
            xs: 12,
            md: 4,
          }}
        >
          <QualityIndicatorsCard nodeDescription={nymNode.description} />
        </Grid2>
        <Grid2
          size={{
            xs: 12,
            md: 6,
          }}
        >
          <NodeRewardsCard rewardDetails={nymNode.rewarding_details} />
        </Grid2>
        <Grid2
          size={{
            xs: 12,
            md: 6,
          }}
        >
          <NodeMetricsCard
            nodeDescription={nymNode.description}
            nodeId={nymNode.bond_information.node_id}
          />
        </Grid2>
      </Grid2>
    </ContentLayout>
  );
}
