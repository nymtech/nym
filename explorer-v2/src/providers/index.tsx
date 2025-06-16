import CosmosKitProvider from "./CosmosKitProvider";
import { EnvironmentProvider } from "./EnvironmentProvider";
import { EpochProvider } from "./EpochProvider";
import { QueryProvider } from "./QueryProvider";
import ThemeProvider from "./ThemeProvider";

const Providers = ({ children }: { children: React.ReactNode }) => {
  return (
    <EnvironmentProvider>
      <ThemeProvider>
        <QueryProvider>
          <EpochProvider>
            <CosmosKitProvider>{children}</CosmosKitProvider>
          </EpochProvider>
        </QueryProvider>
      </ThemeProvider>
    </EnvironmentProvider>
  );
};

export default Providers;
