import CosmosKitProvider from "./CosmosKitProvider";
import { QueryProvider } from "./QueryProvider";
import ThemeProvider from "./ThemeProvider";

const Providers = ({ children }: { children: React.ReactNode }) => {
  return (
    <ThemeProvider>
      <QueryProvider>
        <CosmosKitProvider>{children}</CosmosKitProvider>
      </QueryProvider>
    </ThemeProvider>
  );
};

export default Providers;
