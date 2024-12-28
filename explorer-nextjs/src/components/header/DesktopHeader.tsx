import NymLogo from "@/components/icons/NymLogo";
import { Link } from "@/components/muiLink";
import { Wrapper } from "@/components/wrapper";
import { Box, Divider } from "@mui/material";
import ConnectWallet from "../wallet/ConnectWallet";
import HeaderItem from "./HeaderItem";

export type MenuItem = {
  id: number;
  title: string;
  url: string;
};

const DUMMY_MENU_DATA: MenuItem[] = [
  {
    id: 1,
    title: "Explorer",
    url: "/explorer",
  },
  {
    id: 2,
    title: "Stake",
    url: "/stake",
  },
  {
    id: 3,
    title: "Onboarding",
    url: "/onboarding",
  },
];

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
          {DUMMY_MENU_DATA.map((menu) => (
            <HeaderItem key={menu.id} menu={menu} />
          ))}
        </Box>
        <ConnectWallet size="small" />
      </Wrapper>
      <Divider variant="fullWidth" sx={{ width: "100%" }} />
    </Box>
  );
};
