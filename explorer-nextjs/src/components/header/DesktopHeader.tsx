"use client";

import NymLogo from "@/components/icons/NymLogo";
import { Link } from "@/components/muiLink";
import { Wrapper } from "@/components/wrapper";
import { subtitles } from "@/theme/typography";
import { Circle } from "@mui/icons-material";
import { Box, Button, Divider, Stack } from "@mui/material";
import { usePathname } from "next/navigation";
import type React from "react";
import ConnectWallet from "../wallet/ConnectWallet";

const DUMMY_MENU_DATA = [
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
  const pathname = usePathname();
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
            <Stack direction="row" gap={1} key={menu.id} alignItems="center">
              {pathname.includes(menu.url) && <Circle sx={{ fontSize: 10 }} />}
              <Button
                href={menu.url}
                sx={{
                  borderRadius: 0,
                  padding: 0,
                  minWidth: "auto",
                  display: "flex",
                  justifyContent: "center",
                  alignItems: "center",
                  gap: 1,
                  height: "100%",
                  ...subtitles.subtitle1,
                }}
              >
                {menu.title}
              </Button>
            </Stack>
          ))}
        </Box>
        <ConnectWallet size="small" />
      </Wrapper>
      <Divider variant="fullWidth" sx={{ width: "100%" }} />
    </Box>
  );
};
