import { ContentLayout } from "@/components/contentLayout/ContentLayout";
import SectionHeading from "@/components/headings/SectionHeading";
import { Box, Typography } from "@mui/material";
import Grid from "@mui/material/Grid2";
import Markdown from "react-markdown";
import AccountNotFoundClient from "./AccountNotFoundClient";

export default async function AccountNotFound({
  params,
}: {
  params: Promise<{ address: string }>;
}) {
  const { address } = await params;

  return (
    <ContentLayout>
      <Grid container columnSpacing={5} rowSpacing={5}>
        <Grid size={12}>
          <Box sx={{ display: "flex", justifyContent: "space-between" }}>
            <SectionHeading title="Nym Node Details" />
            <AccountNotFoundClient address={address} />
          </Box>
        </Grid>
      </Grid>
      <Typography variant="h5">
        <Markdown className="reactMarkDownLink">
          This account doesn&apos;t have a Nym node bonded. Is this your
          account? Start [setting up your node](https://nym.com/docs) today!
        </Markdown>
      </Typography>
    </ContentLayout>
  );
}
