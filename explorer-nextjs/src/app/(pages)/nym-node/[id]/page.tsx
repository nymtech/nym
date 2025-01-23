import type { ExplorerData } from "@/app/api";
import type { IObservatoryNode } from "@/app/api/types";
import {
  CURRENT_EPOCH_REWARDS,
  DATA_OBSERVATORY_NODES_URL,
} from "@/app/api/urls";
import BlogArticlesCards from "@/components/blogs/BlogArticleCards";
import ExplorerCard from "@/components/cards/ExplorerCard";
import { ContentLayout } from "@/components/contentLayout/ContentLayout";
import SectionHeading from "@/components/headings/SectionHeading";
import { BasicInfoCard } from "@/components/nymNodePageComponents/BasicInfoCard";
import { NodeChatCard } from "@/components/nymNodePageComponents/ChatCard";
import DelegationsTable from "@/components/nymNodePageComponents/DelegationsTable";
import NodeDelegationsCard from "@/components/nymNodePageComponents/NodeDelegationsCard";
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

    if (!observatoryNymNode) {
      return null;
    }

    return (
      <ContentLayout>
        <Grid container columnSpacing={5} rowSpacing={5}>
          <Grid size={12}>
            <Box sx={{ display: "flex", justifyContent: "space-between" }}>
              <SectionHeading title="Nym Node Details" />
              {observatoryNymNode.bonding_address && (
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
              )}
            </Box>
          </Grid>
          <Grid
            size={{
              xs: 12,
              md: 4,
            }}
          >
            <NodeProfileCard id={id} />
          </Grid>
          <Grid
            size={{
              xs: 12,
              md: 4,
            }}
          >
            <BasicInfoCard id={id} />
          </Grid>
          <Grid
            size={{
              xs: 12,
              md: 4,
            }}
          >
            <QualityIndicatorsCard id={id} />
          </Grid>
          <Grid
            size={{
              xs: 12,
              md: 6,
            }}
          >
            <NodeRewardsCard id={id} />
          </Grid>
          <Grid
            size={{
              xs: 12,
              md: 6,
            }}
          >
            <NodeMetricsCard id={id} />
          </Grid>
          <Grid
            size={{
              xs: 12,
            }}
          >
            <NodeDelegationsCard id={id} />
          </Grid>

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
