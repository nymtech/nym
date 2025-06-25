"use client";

import { Box, Divider, Stack, Button } from "@mui/material";
import NymLogo from "../../components/icons/NymLogo";
import { Link } from "../../components/muiLink";
import { Wrapper } from "../../components/wrapper";
import ConnectWallet from "../wallet/ConnectWallet";
import { DarkLightSwitchDesktop } from "./Switch";
import MENU_DATA from "./menuItems";
import { EnvironmentSwitcher } from "./EnvironmentSwitcher";
import { usePathname } from "next/navigation";
import { getBasePathByEnv } from "../../../envs/config";
import { useEnvironment } from "@/providers/EnvironmentProvider";
import { Circle } from "@mui/icons-material";

export const DesktopHeader = () => {
  const pathname = usePathname();
  const { environment } = useEnvironment();
  const basePath = getBasePathByEnv(environment || "mainnet");
  const explorerName = environment
    ? `${environment} Explorer`
    : "Mainnet Explorer";

  // Helper function to determine if a tab is active
  const isTabActive = (tabTitle: string) => {
    // Check if the current pathname matches the tab title
    // For explorerName, check if we're on the base path
    if (tabTitle === explorerName) {
      return pathname === basePath || pathname === basePath + "/";
    }

    // For menu items, check if the pathname includes the menu URL
    const menuItem = MENU_DATA.find((menu) => menu.title === tabTitle);
    if (menuItem) {
      return pathname.includes(menuItem.url);
    }

    return false;
  };

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
          gap: "30px",
          height: "100%",
        }}
      >
        <Link
          href={"https://nym.com/"}
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
            justifyContent: "center",
            height: "100%",
            gap: 4,
          }}
        >
          <Box
            sx={{
              display: "flex",
              alignItems: "center",
              justifyContent: "start",
              height: "100%",
              gap: 1,
            }}
          >
            <Circle
              sx={{
                fontSize: 10,
                opacity: isTabActive(explorerName) ? 1 : 0,
              }}
            />
            <Link href={basePath} passHref style={{ textDecoration: "none" }}>
              <Button
                sx={{
                  padding: 0,
                }}
              >
                {explorerName}
              </Button>
            </Link>
          </Box>
          {MENU_DATA.map((menu) => (
            <Stack direction="row" gap={1} key={menu.id} alignItems="center">
              <Circle
                sx={{
                  fontSize: 10,
                  opacity: isTabActive(menu.title) ? 1 : 0,
                }}
              />

              <Link
                href={`${basePath}${menu.url}`}
                style={{ textDecoration: "none" }}
                passHref
              >
                <Button
                  sx={{
                    padding: 0,
                  }}
                >
                  {menu.title}
                </Button>
              </Link>
            </Stack>
          ))}
        </Box>
        <EnvironmentSwitcher />
        <ConnectWallet size="small" />
        <DarkLightSwitchDesktop />
      </Wrapper>
      <Divider variant="fullWidth" sx={{ width: "100%" }} />
    </Box>
  );
};
