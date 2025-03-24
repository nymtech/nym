import { ContentLayout } from "@/components/contentLayout/ContentLayout";
import SectionHeading from "@/components/headings/SectionHeading";
import { BasicInfoCard } from "@/components/nymNodePageComponents/BasicInfoCard";
import { NodeDataCard } from "@/components/nymNodePageComponents/NodeDataCard";
// import { NodeChatCard } from "@/components/nymNodePageComponents/ChatCard";
import NodeDelegationsCard from "@/components/nymNodePageComponents/NodeDelegationsCard";
import NodePageButtonGroup from "@/components/nymNodePageComponents/NodePageButtonGroup";
import { NodeParametersCard } from "@/components/nymNodePageComponents/NodeParametersCard";
import { NodeProfileCard } from "@/components/nymNodePageComponents/NodeProfileCard";
import { NodeRoleCard } from "@/components/nymNodePageComponents/NodeRoleCard";
import { Box } from "@mui/material";
import Grid from "@mui/material/Grid2";

export default async function NymNode({
  params,
}: {
  params: Promise<{ id: string }>; // node_id or identity_key
}) {
  try {
    const paramsId = (await params).id;
    const id = Number(paramsId);

    return (
      <ContentLayout>
        <Grid container columnSpacing={5} rowSpacing={5}>
          <Grid size={12}>
            <Box sx={{ display: "flex", justifyContent: "space-between" }}>
              <SectionHeading title="Nym Node Details" />
              <NodePageButtonGroup paramId={id.toString()} />
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
            <NodeRoleCard id={id} />
          </Grid>
          <Grid
            size={{
              xs: 12,
              md: 6,
            }}
          >
            <NodeParametersCard id={id} />
          </Grid>
          <Grid
            size={{
              xs: 12,
              md: 6,
            }}
          >
            <NodeDataCard id={id} />
          </Grid>
          <Grid
            size={{
              xs: 12,
            }}
          >
            <NodeDelegationsCard id={id} />
          </Grid>
          {/* 
          <Grid
            size={{
              xs: 12,
            }}
          >
            <NodeChatCard />
          </Grid> */}
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
