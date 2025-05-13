"use client";

import { QueryContextProvider } from "@/context/queryContext";
import LayoutWithNav from "@/layouts/LayoutWithNav";
import AppTheme from "@/theme";
import { AppRouterCacheProvider } from "@mui/material-nextjs/v15-appRouter";
import CssBaseline from "@mui/material/CssBaseline";
import InitColorSchemeScript from "@mui/material/InitColorSchemeScript";
import { AdapterDayjs } from "@mui/x-date-pickers/AdapterDayjs";
import { LocalizationProvider } from "@mui/x-date-pickers/LocalizationProvider";

export default function RootLayout(props: { children: React.ReactNode }) {
  return (
    <html lang="en" suppressHydrationWarning>
      <body>
        <InitColorSchemeScript attribute="class" />
        <AppRouterCacheProvider options={{ enableCssLayer: true }}>
          <AppTheme>
            {/* CssBaseline kickstart an elegant, consistent, and simple baseline to build upon. */}
            <CssBaseline enableColorScheme />
            <LocalizationProvider dateAdapter={AdapterDayjs}>
              <QueryContextProvider>
                <LayoutWithNav>{props.children}</LayoutWithNav>
              </QueryContextProvider>
            </LocalizationProvider>
          </AppTheme>
        </AppRouterCacheProvider>
      </body>
    </html>
  );
}
