import ClientThemeWrapper from "./ClientThemeWrapper";

const ThemeProvider = ({ children }: { children: React.ReactNode }) => {
  return <ClientThemeWrapper>{children}</ClientThemeWrapper>;
};

export default ThemeProvider;
