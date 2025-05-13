import { ThemeProvider, createTheme } from "@mui/material/styles";
import * as React from "react";

interface AppThemeProps {
  children: React.ReactNode;
  // themeComponents?: ThemeOptions["components"];
}

export default function AppTheme(props: AppThemeProps) {
  const { children } = props;
  const theme = React.useMemo(() => {
    return createTheme({
      colorSchemes: {
        dark: true,
        light: true,
      },
      typography: {
        fontFamily: "system-ui, sans-serif",
      },
    });
  }, []);
  return (
    <ThemeProvider theme={theme} disableTransitionOnChange>
      {children}
    </ThemeProvider>
  );
}
