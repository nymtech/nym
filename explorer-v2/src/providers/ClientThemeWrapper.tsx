"use client";

import dynamic from "next/dynamic";

const ClientThemeProvider = dynamic(() => import("./ClientThemeProvider"), {
  ssr: false,
});

const ClientThemeWrapper = ({ children }: { children: React.ReactNode }) => {
  return <ClientThemeProvider>{children}</ClientThemeProvider>;
};

export default ClientThemeWrapper;
