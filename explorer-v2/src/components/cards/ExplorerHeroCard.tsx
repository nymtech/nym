"use client";
import {
  Box,
  Card,
  CardContent,
  CardHeader,
  Stack,
  type SxProps,
  Typography,
  useTheme,
} from "@mui/material";
import Image from "next/image";
import ArrowUpRight from "../../components/icons/ArrowUpRight";
import { Link } from "../muiLink";

const cardStyles = {
  p: 3,
  cursor: "pointer",
  "&:hover": {
    // Hover style adjusted based on theme mode below
  },
};
const cardContentStyles = {
  mt: 10,
};
const titleStyles = {
  letterSpacing: 0.7,
};

const ExplorerHeroCard = ({
  title,
  label,
  description,
  iconLightSrc,
  iconDarkSrc,
  link,
  sx,
}: {
  title: string;
  label: string;
  description: string;
  iconLightSrc: string;
  iconDarkSrc: string;
  link: string;
  sx?: SxProps;
}) => {
  const theme = useTheme();
  const isDarkMode = theme.palette.mode === "dark";

  const dynamicCardStyles = {
    ...cardStyles,
    bgcolor: isDarkMode ? "#EFFFF0" : "background.paper",
    "&:hover": {
      bgcolor: isDarkMode ? "#C2FFC7" : "#E5E7EB",
    },
    ...sx,
  };

  const dynamicTitleStyles = {
    ...titleStyles,
    color: isDarkMode ? "pine.950" : "inherit",
  };

  const dynamicDescriptionStyles = {
    color: isDarkMode ? "pine.950" : "inherit",
  };

  const iconSrc = isDarkMode ? iconDarkSrc : iconLightSrc;

  return (
    <Link
      href={link}
      sx={{ textDecoration: "none", height: "100%" }}
      target="_blank"
      rel="noopener noreferrer"
    >
      <Card sx={dynamicCardStyles} elevation={0}>
        <CardHeader
          title={
            <Stack direction="row" justifyContent="space-between">
              <Typography variant="body4" sx={dynamicTitleStyles}>
                {label}
              </Typography>
              <Box sx={{ color: isDarkMode ? "pine.950" : "inherit" }}>
                <ArrowUpRight />
              </Box>
            </Stack>
          }
        />
        <CardContent sx={cardContentStyles}>
          <Stack spacing={4}>
            <Image
              src={iconSrc}
              alt={"explorer-blog-image"}
              width={84}
              height={84}
            />
            <Typography variant="h2" sx={dynamicTitleStyles}>
              {title}
            </Typography>
            <Typography variant="body3" sx={dynamicDescriptionStyles}>
              {description}
            </Typography>
          </Stack>
        </CardContent>
      </Card>
    </Link>
  );
};
export default ExplorerHeroCard;
