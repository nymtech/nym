import { Typography } from "@mui/material";
import * as React from "react";

export const ComponentError = ({ text }: { text: string }) => (
  <Typography
    sx={{ marginTop: 2, color: "primary.main", fontSize: 10 }}
    variant="body1"
    data-testid="delegation-total-amount"
  >
    {text}
  </Typography>
);
