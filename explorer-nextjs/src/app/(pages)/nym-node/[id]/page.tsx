import type NodeData from "@/app/api/types";
import { NYM_NODES } from "@/app/api/urls";
import { ContentLayout } from "@/components/contentLayout/ContentLayout";
import SectionHeading from "@/components/headings/SectionHeading";
import { BasicInfoCard } from "@/components/nymNodePageComponents/BasicInfoCard";
import { NodeMetricsCard } from "@/components/nymNodePageComponents/NodeMetricsCard";
import { NodeProfileCard } from "@/components/nymNodePageComponents/NodeProfileCard";
import { NodeRewardsCard } from "@/components/nymNodePageComponents/NodeRewardsCard";
import { QualityIndicatorsCard } from "@/components/nymNodePageComponents/QualityIndicatorsCard";
import ExplorerButtonGroup from "@/components/toggleButton/ToggleButton";
import { Box, Grid2 } from "@mui/material";

export default async function NymNode({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  try {
    const id = Number((await params).id);

    const response = await fetch(NYM_NODES, {
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json; charset=utf-8",
      },
      next: { revalidate: 60 },
      // refresh event list cache at given interval
    });

    const nymNodes: NodeData[] = await response.json();

    if (!nymNodes) {
      return null;
    }

    const nymNode = nymNodes.find((node) => node.node_id === id);

    if (!nymNode) {
      return null;
    }

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
          <Grid2
            size={{
              xs: 12,
              md: 4,
            }}
          >
            <BasicInfoCard
              bondInfo={nymNode.bond_information}
              nodeDescription={nymNode.description}
              rewardDetails={nymNode.rewarding_details}
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
  } catch (error) {
    let errorMessage = "An error occurred";
    if (error instanceof Error) {
      errorMessage = error.message;
    }
    throw new Error(errorMessage);
  }
}
