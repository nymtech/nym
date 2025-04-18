"use client";
import { Close as CloseIcon, Menu as MenuIcon } from "@mui/icons-material";
import { Box, Drawer, IconButton, Typography } from "@mui/material";
import { useTheme } from "@mui/material/styles";
import { useState } from "react";
import { Link } from "../../components/muiLink";
import { Wrapper } from "../../components/wrapper";
import NymLogo from "../icons/NymLogo";
import ConnectWallet from "../wallet/ConnectWallet";
import { DarkLightSwitchDesktop } from "./Switch";
import MENU_DATA from "./menuItems";

export const MobileHeader = () => {
  const [drawerOpen, setDrawerOpen] = useState(false);
  const theme = useTheme();

  // Mobile menu handlers
  const toggleDrawer = (open: boolean) => {
    setDrawerOpen(open);
  };

  return (
    <>
      <Box sx={{ display: { xs: "block", lg: "none" } }}>
        <MobileMenuHeader toggleDrawer={toggleDrawer} drawerOpen={drawerOpen} />
        <Drawer
          anchor="left"
          open={drawerOpen}
          onClose={() => toggleDrawer(false)}
          sx={{
            display: { xs: "block", lg: "none" },
          }}
          PaperProps={{
            sx: {
              width: "100%",
              height: "100%",
              bgcolor:
                theme.palette.mode === "dark" ? "pine.900" : "base.white",
              overflow: "hidden",
              position: "relative",
              color:
                theme.palette.mode === "dark"
                  ? "base.white"
                  : "background.main",
            },
          }}
        >
          <MobileMenuHeader
            toggleDrawer={toggleDrawer}
            drawerOpen={drawerOpen}
          />
          {/* Sliding Animation */}
          <Box
            sx={{
              display: "flex",
              width: "200%",
              height: "100%",
              transition: "transform 0.3s ease-in-out",
              position: "relative",
              transform: "translateX(0%)",
            }}
          >
            {/* Main Menu */}
            <Box sx={{ width: "50%", height: "100%" }}>
              {MENU_DATA.map((menu) => (
                <Box key={menu.title} sx={{ marginBottom: 3 }}>
                  <Link
                    onClick={() => toggleDrawer(false)}
                    href={menu.url || ""}
                    target={menu?.url?.startsWith("http") ? "_blank" : "_self"}
                    sx={{
                      display: "flex",
                      width: "100%",
                      padding: 3.75,
                      color:
                        theme.palette.mode === "dark"
                          ? "base.white"
                          : "background.main",
                      justifyContent: "space-between",
                      alignItems: "center",
                    }}
                  >
                    <Box
                      sx={{
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "flex-start",
                        gap: 1.25,
                      }}
                    >
                      <Box
                        sx={{
                          display: "block",
                          width: "10px",
                          height: "10px",
                          borderRadius: "100%",
                          bgcolor:
                            theme.palette.mode === "dark"
                              ? "base.white"
                              : "primary.main",
                        }}
                      />
                      <Typography
                        color={
                          theme.palette.mode === "dark"
                            ? "base.white"
                            : "primary"
                        }
                        variant="h4"
                      >
                        {menu.title}
                      </Typography>
                    </Box>
                  </Link>
                </Box>
              ))}
            </Box>
          </Box>
        </Drawer>
      </Box>
    </>
  );
};

const MobileMenuHeader = ({
  toggleDrawer,
  drawerOpen,
}: {
  toggleDrawer: (open: boolean) => void;
  drawerOpen: boolean;
}) => {
  return (
    <Wrapper
      sx={{
        backgroundColor: "background.default",
      }}
    >
      <Box
        sx={{
          height: "115px",
          alignItems: "center",
          display: "flex",
          justifyContent: "space-between",
        }}
      >
        <Link
          onClick={() => toggleDrawer(false)}
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
            alignItems: "center",
          }}
        >
          {!drawerOpen && <DarkLightSwitchDesktop />}
          <IconButton sx={{}} onClick={() => toggleDrawer(!drawerOpen)}>
            {drawerOpen ? <CloseIcon /> : <MenuIcon />}
          </IconButton>
        </Box>
      </Box>
      <Box
        sx={{
          display: "flex",
          flexDirection: "column",
          gap: 2.5,
          justifyContent: "center",
          alignItems: "center",
          width: "100%",
        }}
      >
        {!drawerOpen && <ConnectWallet size="small" />}
      </Box>
      <Box height={40} />
    </Wrapper>
  );
};
