import CosmosKitProvider from "./CosmosKitProvider";
import { QueryProvider } from "./QueryProvider";
import ThemeProvider from "./ThemeProvider";

const Providers = ({ children }: { children: React.ReactNode }) => {
  return (
    <ThemeProvider>
      <CosmosKitProvider>
        <QueryProvider>{children}</QueryProvider>
      </CosmosKitProvider>
    </ThemeProvider>
  );
};

export default Providers;
