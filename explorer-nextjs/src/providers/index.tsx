import CosmosKitProvider from "./CosmosKitProvider";
import { EpochProvider } from "./EpochProvider";
import { QueryProvider } from "./QueryProvider";
import ThemeProvider from "./ThemeProvider";

const Providers = ({ children }: { children: React.ReactNode }) => {
  return (
    <ThemeProvider>
      <QueryProvider>
        <EpochProvider>
          <CosmosKitProvider>{children}</CosmosKitProvider>
        </EpochProvider>
      </QueryProvider>
    </ThemeProvider>
  );
};

export default Providers;
