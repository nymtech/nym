// import BlogArticlesCards from "@/components/blogs/BlogArticleCards";
import { ContentLayout } from "@/components/contentLayout/ContentLayout";
import SectionHeading from "@/components/headings/SectionHeading";
import ExplorerButtonGroup from "@/components/toggleButton/ToggleButton";
import { Box, Typography } from "@mui/material";
import Grid from "@mui/material/Grid2";
import Markdown from "react-markdown";

export default async function Account({
  params,
}: {
  params: Promise<{ address: string }>;
}) {
  try {
    const address = (await params).address;

    return (
      <ContentLayout>
        <Grid container columnSpacing={5} rowSpacing={5}>
          <Grid size={12}>
            <Box sx={{ display: "flex", justifyContent: "space-between" }}>
              <SectionHeading title="Nym Node Details" />
              <ExplorerButtonGroup
                onPage="Account"
                options={[
                  {
                    label: "Nym Node",
                    isSelected: true,
                    link: `/account/${address}/not-found/`,
                  },
                  {
                    label: "Account",
                    isSelected: false,
                    link: `/account/${address}`,
                  },
                ]}
              />
            </Box>
          </Grid>
        </Grid>
        <Typography variant="h5">
          <Markdown className="reactMarkDownLink">
            This account does’t have a Nym node bonded. Is this your account?
            Start [setting up your node](https://nym.com/docs) today!
          </Markdown>
        </Typography>
        {/* <Grid container columnSpacing={5} rowSpacing={5}>
          <Grid size={12}>
            <SectionHeading title="Onboarding" />
          </Grid>
          <BlogArticlesCards ids={[1]} />
        </Grid> */}
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
