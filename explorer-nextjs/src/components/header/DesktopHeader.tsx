"use client";

import { Box, Button, Divider } from "@mui/material";
import type React from "react";
import { Wrapper } from "@/components/wrapper";
import { Link } from "@/components/muiLink";
import NymLogo from "@/components/icons/NymLogo";
import { subtitles } from "@/theme/typography";

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
            <Button
              key={menu.title}
              href={menu.url}
              sx={{
                borderRadius: 0,
                padding: 0,
                minWidth: "auto",
                display: "flex",
                justifyContent: "center",
                alignItems: "center",
                gap: "10px",
                height: "100%",
                ...subtitles.subtitle1,
                "& .MuiButton-endIcon": {
                  marginLeft: 0,
                  marginRight: 0,
                },
                "& .MuiButton-startIcon": {
                  marginLeft: 0,
                  marginRight: 0,
                },
                "&:hover": {
                  textDecoration: "none",
                },
              }}
            >
              {menu.title}
            </Button>
          ))}
        </Box>
        <Button variant="contained" size="small">
          Connect Wallet
        </Button>
      </Wrapper>
      <Divider variant="fullWidth" sx={{ width: "100%" }} />
    </Box>
  );
};
