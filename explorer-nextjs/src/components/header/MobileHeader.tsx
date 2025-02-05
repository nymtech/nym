"use client";
import { Close as CloseIcon, Menu as MenuIcon } from "@mui/icons-material";
import { Box, Drawer, IconButton, Typography } from "@mui/material";
import { useState } from "react";
import { Link } from "../../components/muiLink";
import { Wrapper } from "../../components/wrapper";
import NymLogo from "../icons/NymLogo";
import ConnectWallet from "../wallet/ConnectWallet";
import MENU_DATA from "./menuItems";

export const MobileHeader = () => {
  const [drawerOpen, setDrawerOpen] = useState(false);

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
              bgcolor: "base.white",
              overflow: "hidden",
              position: "relative",
              color: "background.main",
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
                      color: "background.main",
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
                          bgcolor: "primary.main",
                        }}
                      />
                      <Typography color="primary" variant="h4">
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
            gap: 2.5,
            alignItems: "center",
          }}
        >
          <IconButton sx={{}} onClick={() => toggleDrawer(!drawerOpen)}>
            {drawerOpen ? <CloseIcon /> : <MenuIcon />}
          </IconButton>
        </Box>
      </Box>
      {!drawerOpen && <ConnectWallet size="small" />}
      <Box height={40} />
    </Wrapper>
  );
};
