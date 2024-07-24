import * as React from "react";
import { Typography } from "@mui/material";

export const Title: FCWithChildren<{ text: string }> = ({ text }) => (
  <Typography
    variant="h5"
    sx={{
      fontWeight: 600,
      color: "text.primary",
    }}
    data-testid={text}
  >
    {text}
  </Typography>
);
