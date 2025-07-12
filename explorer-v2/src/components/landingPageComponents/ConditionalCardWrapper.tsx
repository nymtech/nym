"use client";
import Grid from "@mui/material/Grid2";
import { ReactNode } from "react";

interface ConditionalCardWrapperProps {
  children: ReactNode;
  size?:
    | number
    | { xs?: number; sm?: number; md?: number; lg?: number; xl?: number };
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
