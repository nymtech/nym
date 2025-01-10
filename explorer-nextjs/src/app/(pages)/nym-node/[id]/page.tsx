import type { IObservatoryNode } from "@/app/api/types";
import { DATA_OBSERVATORY_NODES_URL } from "@/app/api/urls";
import BlogArticlesCards from "@/components/blogs/BlogArticleCards";
import ExplorerCard from "@/components/cards/ExplorerCard";
import { ContentLayout } from "@/components/contentLayout/ContentLayout";
import SectionHeading from "@/components/headings/SectionHeading";
import DelegationsTable from "@/components/nodeTable/DelegationsTable";
import { BasicInfoCard } from "@/components/nymNodePageComponents/BasicInfoCard";
import { NodeChatCard } from "@/components/nymNodePageComponents/ChatCard";
import { NodeMetricsCard } from "@/components/nymNodePageComponents/NodeMetricsCard";
import { NodeProfileCard } from "@/components/nymNodePageComponents/NodeProfileCard";
import { NodeRewardsCard } from "@/components/nymNodePageComponents/NodeRewardsCard";
import { QualityIndicatorsCard } from "@/components/nymNodePageComponents/QualityIndicatorsCard";
import ExplorerButtonGroup from "@/components/toggleButton/ToggleButton";
import { Box } from "@mui/material";
import Grid from "@mui/material/Grid2";

export default async function NymNode({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  try {
    const id = Number((await params).id);

    const observatoryResponse = await fetch(DATA_OBSERVATORY_NODES_URL, {
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json; charset=utf-8",
      },
      next: { revalidate: 60 },
      // refresh event list cache at given interval
    });

    const observatoryNymNodes: IObservatoryNode[] =
      await observatoryResponse.json();

    if (!observatoryNymNodes) {
      return null;
    }

    const observatoryNymNode = observatoryNymNodes.find(
      (node) => node.node_id === id,
    );

    console.log("observatorynNymNode :>> ", observatoryNymNode);

    if (!observatoryNymNode) {
      return null;
    }

    const nodeDelegationsResponse = await fetch(
      `${DATA_OBSERVATORY_NODES_URL}/${id}/delegations`,
      {
        headers: {
          Accept: "application/json",
          "Content-Type": "application/json; charset=utf-8",
        },
        next: { revalidate: 60 },
        // refresh event list cache at given interval
      },
    );

    const delegations = await nodeDelegationsResponse.json();

    return (
      <ContentLayout>
        <Grid container columnSpacing={5} rowSpacing={5}>
          <Grid size={12}>
            <Box sx={{ display: "flex", justifyContent: "space-between" }}>
              <SectionHeading title="Nym Node Details" />
              <ExplorerButtonGroup
                options={[
                  {
                    label: "Nym Node",
                    isSelected: true,
                    link: `/nym-node/${id}`,
                  },
                  {
                    label: "Account",
                    isSelected: false,
                    link: `/account/${observatoryNymNode.bonding_address}`,
                  },
                ]}
              />
            </Box>
          </Grid>
          {observatoryNymNode && (
            <Grid
              size={{
                xs: 12,
                md: 4,
              }}
            >
              <NodeProfileCard nodeInfo={observatoryNymNode} />
            </Grid>
          )}
          {observatoryNymNode && (
            <Grid
              size={{
                xs: 12,
                md: 4,
              }}
            >
              <BasicInfoCard
                rewardDetails={observatoryNymNode.rewarding_details}
                nodeInfo={observatoryNymNode}
              />
            </Grid>
          )}
          {observatoryNymNode && (
            <Grid
              size={{
                xs: 12,
                md: 4,
              }}
            >
              <QualityIndicatorsCard nodeInfo={observatoryNymNode} />
            </Grid>
          )}
          <Grid
            size={{
              xs: 12,
              md: 6,
            }}
          >
            <NodeRewardsCard
              rewardDetails={observatoryNymNode.rewarding_details}
              nodeInfo={observatoryNymNode}
            />
          </Grid>
          {observatoryNymNode && (
            <Grid
              size={{
                xs: 12,
                md: 6,
              }}
            >
              <NodeMetricsCard nodeInfo={observatoryNymNode} />
            </Grid>
          )}
          {delegations && (
            <Grid
              size={{
                xs: 12,
              }}
            >
              <ExplorerCard label="Delegations" sx={{ height: "100%" }}>
                <DelegationsTable delegations={delegations} />
              </ExplorerCard>
            </Grid>
          )}

          <Grid
            size={{
              xs: 12,
            }}
          >
            <NodeChatCard />
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
    let errorMessage = "An error occurred";
    if (error instanceof Error) {
      errorMessage = error.message;
    }
    throw new Error(errorMessage);
  }
}
