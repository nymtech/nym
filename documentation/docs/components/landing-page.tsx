import React from "react";
import { Box, Grid, Typography } from "@mui/material";
import Image from "next/image";
import Link from "next/link";

import networkDocs from "./images/network-docs.png";
import developerDocs from "./images/developer-docs.png";
import sdkDocs from "./images/sdk-docs.png";
import operatorGuide from "./images/operator-guide.png";

export const LandingPage = () => {
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
            md={6}
            padding={4}
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
              <Box display={"flex"} gap={4} height={"100%"}>
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
                    {square.description}
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

      <style jsx>{`
        .landing-page-container {
          max-width: 1200px;
          margin: 0 auto;
          padding: 2rem 1rem;
        }
        .landing-page-intro {
          font-size: 1.125rem;
          margin-bottom: 2rem;
        }
        .landing-page-grid {
          display: grid;
          grid-template-columns: repeat(2, 1fr);
          gap: 2rem;
          max-width: 36rem;
          margin: 0 auto;
        }
        .landing-page-square {
          background-color: #000000;
          color: white;
          padding: 1rem;
          border-radius: 0.5rem;
          text-align: center;
          cursor: pointer;
          text-decoration: none;
          transition: box-shadow 0.3s ease;
          aspect-ratio: 1 / 1;
          display: flex;
          flex-direction: column;
          align-items: center;
          justify-content: center;
          border: 3px solid #ff6600;
          box-shadow: 0 0 10px #ff6600;
        }
        .landing-page-square:hover {
          box-shadow: 0 0 20px #ff6600;
        }
        .landing-page-icon {
          width: 12.5rem;
          height: 12.5rem;
          margin-bottom: 0.75rem;
          color: #ff6600;
        }
        .landing-page-text {
          font-size: 0.875rem;
          font-weight: 600;
          color: #ff6600;
        }
      `}</style>
    </Box>
  );
};
