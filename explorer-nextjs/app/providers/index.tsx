import React from "react";
import CosmosKitProvider from "@/app/context/cosmos-kit";
import { WalletProvider } from "@/app/context/wallet";
import { NetworkExplorerThemeProvider } from "@/app/theme";
import { MainContextProvider } from "@/app/context/main";

const Providers = ({ children }: { children: React.ReactNode }) => {
  return (
    <MainContextProvider>
      <NetworkExplorerThemeProvider>
        <CosmosKitProvider>
          <WalletProvider>{children}</WalletProvider>
        </CosmosKitProvider>
      </NetworkExplorerThemeProvider>
    </MainContextProvider>
  );
};

export { Providers };
