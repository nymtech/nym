"use client";

import { useState, useEffect } from "react";
import { Box, Typography, Button, IconButton, Stack } from "@mui/material";
import { Close, Launch } from "@mui/icons-material";
import { Link } from "../muiLink";
import { Wrapper } from "../wrapper";
import { getBanner } from "@/app/features/banner/api/getBanner";
import type { components } from "@/app/lib/strapi";

type BannerData = {
  id?: number;
  attributes?: components["schemas"]["ExplorerBanner"];
} | null;

export const Banner = () => {
  const [bannerData, setBannerData] = useState<BannerData>(null);
  const [isVisible, setIsVisible] = useState(true);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    const fetchBanner = async () => {
      try {
        const data = await getBanner("en");
        setBannerData(data);
        setIsLoading(false);
      } catch (error) {
        console.error("Failed to fetch banner:", error);
        setIsLoading(false);
      }
    };

    fetchBanner();
  }, []);

  const handleClose = () => {
    setIsVisible(false);
  };

  // Don't render if not visible
  if (!isVisible) {
    return null;
  }

  // Only show banner if API data is available and show is true
  const shouldShowBanner = bannerData?.attributes?.show === true;
  const hasValidData = bannerData?.attributes && shouldShowBanner;

  // If loading and no data yet, don't show anything
  if (isLoading && !bannerData) {
    return null;
  }

  // If API data says don't show and we have valid data, don't show
  if (hasValidData && !shouldShowBanner) {
    return null;
  }

  // If no valid data from API, don't show anything
  if (!hasValidData) {
    return null;
  }

  const { title, text, links, icon } = bannerData.attributes || {};

  return (
    <Box
      sx={{
        backgroundColor: "accent.main",
        color: "base.black",
        py: { xs: 1.5, md: 2 },
        borderBottom: "1px solid",
        borderColor: "divider",
      }}
    >
      <Wrapper>
        <Stack
          direction={{ xs: "column", md: "row" }}
          alignItems={{ xs: "flex-start", md: "center" }}
          justifyContent="space-between"
          spacing={{ xs: 1.5, md: 2 }}
        >
          <Stack
            direction="row"
            alignItems="center"
            spacing={3}
            flex={1}
            sx={{ width: "100%" }}
          >
            {/* Icon */}
            {icon?.data?.attributes?.url && (
              <Box
                component="img"
                src={icon.data.attributes.url}
                alt={icon.data.attributes.alternativeText || "Banner icon"}
                sx={{
                  width: { xs: 20, md: 24 },
                  height: { xs: 20, md: 24 },
                  flexShrink: 0,
                }}
              />
            )}

            {/* Content */}
            <Box flex={1}>
              <Typography
                variant="subtitle1"
                sx={{
                  fontWeight: 600,
                  mb: 0.5,
                  fontSize: { xs: "0.875rem", md: "1rem" },
                }}
              >
                {title}
              </Typography>
              <Typography
                variant="body2"
                sx={{
                  opacity: 0.9,
                  fontSize: { xs: "0.75rem", md: "0.875rem" },
                }}
              >
                {text}
              </Typography>
            </Box>
          </Stack>

          {/* Actions */}
          <Stack
            direction="row"
            alignItems="center"
            spacing={1}
            sx={{
              alignSelf: { xs: "flex-end", md: "center" },
              width: { xs: "auto", md: "auto" },
            }}
          >
            {/* Links */}
            {links && links.length > 0 && (
              <Stack
                direction={{ xs: "column", sm: "row" }}
                spacing={1}
                sx={{ flexWrap: "wrap" }}
              >
                {links.map((link) => (
                  <Link
                    key={link.id}
                    href={link.url || "#"}
                    target={link.url?.startsWith("http") ? "_blank" : "_self"}
                    rel={
                      link.url?.startsWith("http") ? "noopener noreferrer" : ""
                    }
                    style={{ textDecoration: "none" }}
                  >
                    <Button
                      variant="outlined"
                      size="small"
                      endIcon={
                        link.url?.startsWith("http") ? <Launch /> : undefined
                      }
                      sx={{
                        color: "base.black",
                        borderColor: "base.black",
                        fontSize: { xs: "0.75rem", md: "0.875rem" },
                        px: { xs: 1, md: 2 },
                        py: { xs: 0.5, md: 1 },
                        "&:hover": {
                          borderColor: "base.black",
                          backgroundColor: "rgba(0, 0, 0, 0.1)",
                        },
                      }}
                    >
                      {link.title}
                    </Button>
                  </Link>
                ))}
              </Stack>
            )}

            {/* Close button */}
            <IconButton
              onClick={handleClose}
              size="small"
              sx={{
                color: "base.black",
                "&:hover": {
                  backgroundColor: "rgba(0, 0, 0, 0.1)",
                },
              }}
            >
              <Close sx={{ fontSize: { xs: "1.25rem", md: "1.5rem" } }} />
            </IconButton>
          </Stack>
        </Stack>
      </Wrapper>
    </Box>
  );
};
