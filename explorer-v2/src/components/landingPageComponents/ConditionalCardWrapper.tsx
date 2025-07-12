"use client";
import Grid from "@mui/material/Grid2";
import { ReactNode } from "react";

interface ConditionalCardWrapperProps {
  children: ReactNode;
  size?: any;
  visible?: boolean;
}

export const ConditionalCardWrapper = ({
  children,
  size,
  visible = true,
}: ConditionalCardWrapperProps) => {
  if (!visible) {
    return null;
  }

  return <Grid size={size}>{children}</Grid>;
};
