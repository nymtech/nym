"use client";

import * as React from "react";
import { NymNetworkExplorerThemeProvider } from "@nymproject/mui-theme";
import { useMainContext } from "../context/main";

export const NetworkExplorerThemeProvider = ({
  children,
}: {
  children: React.ReactNode;
}) => {
  const { mode } = useMainContext();

  return (
    <NymNetworkExplorerThemeProvider mode={mode}>
      {children}
    </NymNetworkExplorerThemeProvider>
  );
};
