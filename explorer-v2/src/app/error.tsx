"use client";

import { ContentLayout } from "@/components/contentLayout/ContentLayout";
import { Link } from "@/components/muiLink";
import { Button, Stack, Typography } from "@mui/material";

const ErrorPage = ({ error }: { error: Error }) => {
  return (
    <ContentLayout>
      <Stack spacing={2} justifyContent="flex-start">
        <Typography variant="body1">
          An error occurred: {error.message}
        </Typography>
        <Typography variant="body2">
          Please try again later or contact support
        </Typography>
        <Link href="/" underline="none">
          <Button variant="contained" sx={{ maxWidth: 100 }} size="small">
            Home
          </Button>
        </Link>
      </Stack>
    </ContentLayout>
  );
};

export default ErrorPage;
