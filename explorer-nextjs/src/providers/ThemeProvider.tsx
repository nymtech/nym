"use client";
import { lightTheme } from "@/theme/theme";
import { CssBaseline } from "@mui/material";
import { ThemeProvider as MUIThemeProvider } from "@mui/material";
import { AppRouterCacheProvider } from "@mui/material-nextjs/v15-appRouter";

const ThemeProvider = ({ children }: { children: React.ReactNode }) => {
  return (
    <AppRouterCacheProvider>
      <MUIThemeProvider theme={lightTheme}>
        <CssBaseline />
        {children}
      </MUIThemeProvider>
    </AppRouterCacheProvider>
  );
};

export default ThemeProvider;
