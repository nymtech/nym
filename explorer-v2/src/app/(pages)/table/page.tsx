// import BlogArticlesCards from "@/components/blogs/BlogArticleCards";
import { ContentLayout } from "@/components/contentLayout/ContentLayout";
import SectionHeading from "@/components/headings/SectionHeading";
import NodeTableWithAction from "@/components/nodeTable/NodeTableWithAction";
import NodeAndAddressSearch from "@/components/search/NodeAndAddressSearch";
import { Wrapper } from "@/components/wrapper";
import { Box, Stack } from "@mui/material";
// import Grid from "@mui/material/Grid2";

import { RECOMMENDED_NODES } from "@/app/constants"; // ⬅ dynamic Promise<number[]>

export default async function ExplorerPage() {
  // Resolve once on the server and pass IDs to client components
  const recommendedIds = await RECOMMENDED_NODES;

  return (
    <ContentLayout>
      <Wrapper>
        <Stack gap={5}>
          <SectionHeading title="Explorer" />
          <NodeAndAddressSearch />
        </Stack>
        <Box sx={{ mt: 5 }}>
          <NodeTableWithAction recommendedIds={recommendedIds} />
        </Box>
        {/* <Grid container columnSpacing={5} rowSpacing={5} mt={10}>
          <Grid size={12}>
            <SectionHeading title="Onboarding" />
          </Grid>
          <BlogArticlesCards limit={2} />
        </Grid> */}
      </Wrapper>
    </ContentLayout>
  );
}
