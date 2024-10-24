import React from "react";
import { Box, Grid, Typography } from "@mui/material";
import useMediaQuery from "@mui/material/useMediaQuery";
import { useTheme } from "@mui/material/styles";

import Image from "next/image";
import Link from "next/link";

import networkDocs from "./images/network-docs.png";
import developerDocs from "./images/developer-docs.png";
import sdkDocs from "./images/sdk-docs.png";
import operatorGuide from "./images/operator-guide.png";
import { t } from "nextra/dist/types-c8e621b7";

export const LandingPage = () => {
  const theme = useTheme();
  const isTablet = useMediaQuery(theme.breakpoints.up("md"));
  const isDesktop = useMediaQuery(theme.breakpoints.up("xl"));

  const squares = [
    {
      text: "Network Docs",
      description: "Architecture, crypto systems, and how the Mixnet works",
      href: "/network",
      icon: developerDocs,
    },
    {
      text: "Operator Guides",
      description:
        "Guides and maintenance: if you want to run a node, start here",

      href: "/operators",
      icon: operatorGuide,
    },
    {
      text: "Developer Portal",
      description:
        "Conceptual overview, clients, and tools for developers and integrations",

      href: "/developers",
      icon: networkDocs,
    },
    {
      text: "SDKs",
      description: "Rust and Typescript SDK docs",

      href: "/developers/rust",
      icon: sdkDocs,
    },
  ];

  const shortenDescription = (description: string) => {
    return description.slice(0, 18) + "...";
  };

  return (
    <Box maxWidth={1200} margin={"0 auto"}>
      <Typography variant="h2" mb={6}>
        Nym Docs
      </Typography>

      <Typography mb={10}>
        Nym is a privacy platform. It provides strong network-level privacy
        against sophisticated end-to-end attackers, and anonymous access control
        using blinded, re-randomizable, decentralized credentials. Our goal is
        to allow developers to build new applications, or upgrade existing apps,
        with privacy features unavailable in other systems.
      </Typography>
      <Grid container border={"1px solid #262626"}>
        {squares.map((square, index) => (
          <Grid
            item
            key={index}
            xs={12}
            lg={6}
            padding={{ xs: 3, xl: 4 }}
            width={"100%"}
            sx={{
              borderBottom: {
                xs: index < 3 ? "1px solid #262626" : "none",
                md: index === 0 || index === 1 ? "1px solid #262626" : "none",
              },
              borderRight: {
                md: index === 0 || index === 2 ? "1px solid #262626" : "none",
              },
            }}
          >
            <Link href={square.href} target="_blank" rel="noopener noreferrer">
              <Box display={"flex"} gap={{ xs: 3, xl: 4 }} height={"100%"}>
                <Image
                  src={square.icon}
                  alt={square.text}
                  width={180}
                  height={134}
                />
                <Box
                  display={"flex"}
                  flexDirection={"column"}
                  justifyContent={"space-between"}
                  flexGrow={1}
                  height={"100%"}
                >
                  <Typography variant="h5" sx={{ fontWeight: 600 }}>
                    {square.text}
                  </Typography>
                  <Typography variant="body1" sx={{ color: "#909195" }}>
                    {isTablet && !isDesktop
                      ? shortenDescription(square.description)
                      : square.description}
                  </Typography>

                  <Typography sx={{ color: "#ff6600", fontWeight: 600 }}>
                    Open
                  </Typography>
                </Box>
              </Box>
            </Link>
          </Grid>
        ))}
      </Grid>
    </Box>
  );
};
