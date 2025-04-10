import { Box, Divider } from "@mui/material";
import NymLogo from "../../components/icons/NymLogo";
import { Link } from "../../components/muiLink";
import { Wrapper } from "../../components/wrapper";
import ConnectWallet from "../wallet/ConnectWallet";
import HeaderItem from "./HeaderItem";
import { DarkLightSwitchDesktop } from "./Switch";
import MENU_DATA from "./menuItems";

export const DesktopHeader = () => {
  return (
    <Box
      sx={{
        display: { xs: "none", lg: "block" },
        height: "115px",
        alignItems: "center",
      }}
    >
      <Wrapper
        sx={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          gap: "42px",
          height: "100%",
        }}
      >
        <Link
          href={"/"}
          style={{
            display: "flex",
            alignItems: "center",
            width: "100px",
            aspectRatio: "89/25",
          }}
        >
          <NymLogo />
        </Link>
        <Box
          sx={{
            display: "flex",
            flexGrow: 1,
            alignItems: "center",
            justifyContent: "start",
            height: "100%",
            gap: 5,
          }}
        >
          {MENU_DATA.map((menu) => (
            <HeaderItem key={menu.id} menu={menu} />
          ))}
        </Box>
        <ConnectWallet size="small" />
        <DarkLightSwitchDesktop defaultChecked />
      </Wrapper>
      <Divider variant="fullWidth" sx={{ width: "100%" }} />
    </Box>
  );
};
