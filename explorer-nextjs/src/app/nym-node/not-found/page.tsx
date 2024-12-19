import { ContentLayout } from "@/components/contentLayout/ContentLayout";
import SectionHeading from "@/components/headings/SectionHeading";
import { Typography } from "@mui/material";

const NotFound = () => {
  return (
    <ContentLayout>
      <SectionHeading title="Nym Node" />
      <Typography variant="body3">This Nym Node could not be found</Typography>
    </ContentLayout>
  );
};

export default NotFound;
