"use client";

import { CssBaseline, PaletteMode } from "@mui/material";
import { ThemeProvider as MUIThemeProvider } from "@mui/material";
import { AppRouterCacheProvider } from "@mui/material-nextjs/v15-appRouter";
import { lightTheme, darkTheme } from "../theme/theme";
import { useLocalStorage } from "@uidotdev/usehooks";
import { useEffect, useState } from "react";

const ClientThemeProvider = ({ children }: { children: React.ReactNode }) => {
  const [isMounted, setIsMounted] = useState(false);
  const [mode] = useLocalStorage<PaletteMode>("mode", "dark");

  useEffect(() => {
    setIsMounted(true);
  }, []);

  if (!isMounted) return null; // or a loading spinner if you prefer

  return (
    <AppRouterCacheProvider>
      <MUIThemeProvider theme={mode === "light" ? lightTheme : darkTheme}>
        <CssBaseline />
        {children}
      </MUIThemeProvider>
    </AppRouterCacheProvider>
  );
};

export default ClientThemeProvider;
