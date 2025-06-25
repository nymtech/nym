// import BlogArticlesCards from "@/components/blogs/BlogArticleCards";
import { ContentLayout } from "@/components/contentLayout/ContentLayout";
import SectionHeading from "@/components/headings/SectionHeading";
import NodeTableWithAction from "@/components/nodeTable/NodeTableWithAction";
import NodeAndAddressSearch from "@/components/search/NodeAndAddressSearch";
import { Wrapper } from "@/components/wrapper";
import { Box, Stack } from "@mui/material";
// import Grid from "@mui/material/Grid2";

export default function ExplorerPage() {
  return (
    <ContentLayout>
      <Wrapper>
        <Stack gap={5}>
          <SectionHeading title="Servers" />
          <NodeAndAddressSearch />
        </Stack>
        <Box sx={{ mt: 5 }}>
          <NodeTableWithAction />
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
