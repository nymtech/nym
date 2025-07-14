import { Box } from "@mui/material";
import { DesktopHeader } from "./DesktopHeader";
import { MobileHeader } from "./MobileHeader";

export const Header = async () => {
  return (
    <Box
      component="header"
      sx={{
        backgroundColor: "background.default",
      }}
    >
      <DesktopHeader />
      <MobileHeader />
      {/* Mobile header will go here */}
    </Box>
  );
};
