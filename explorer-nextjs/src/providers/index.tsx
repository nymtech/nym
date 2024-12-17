import CosmosKitProvider from "./CosmosKitProvider";
import ThemeProvider from "./ThemeProvider";

const Providers = ({ children }: { children: React.ReactNode }) => {
  return (
    <ThemeProvider>
      <CosmosKitProvider>{children}</CosmosKitProvider>
    </ThemeProvider>
  );
};

export default Providers;
