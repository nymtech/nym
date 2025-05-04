"use client";
import React from "react";
import {
  Box,
  Button,
  Collapse,
  Slider,
  Typography,
  useTheme,
} from "@mui/material";
import Grid from "@mui/material/Grid2";

import FilterAltIcon from "@mui/icons-material/FilterAlt";
import AccessTimeIcon from "@mui/icons-material/AccessTime";
import PieChartIcon from "@mui/icons-material/PieChart";
import PercentIcon from "@mui/icons-material/Percent";

type AdvancedFiltersProps = {
  uptime: [number, number];
  setUptime: (value: [number, number]) => void;
  saturation: [number, number];
  setSaturation: (value: [number, number]) => void;
  profitMargin: [number, number];
  setProfitMargin: (value: [number, number]) => void;
  open?: boolean;
  setOpen?: (open: boolean) => void;
  onlyPanel?: boolean;
  maxSaturation?: number;
};

export default function AdvancedFilters({
  uptime,
  setUptime,
  saturation,
  setSaturation,
  profitMargin,
  setProfitMargin,
  open,
  setOpen,
  onlyPanel,
  maxSaturation = 100,
}: AdvancedFiltersProps) {
  const theme = useTheme();
  const green = "#14e76f"; // from theme colours

  const marksPercent: { value: number }[] = [{ value: 0 }, { value: 100 }];
  const marksSaturation: { value: number }[] = [
    { value: 0 },
    { value: maxSaturation },
  ];

  const panel = (
    <Box
      sx={{
        mt: onlyPanel ? 0 : 3,
        p: 2,
        borderRadius: 3,
        background: theme.palette.background.paper,
        border: `1px solid ${theme.palette.divider}`,
      }}
    >
      <Typography
        variant="subtitle1"
        sx={{
          fontStyle: "italic",
          color:
            theme.palette.mode === "light"
              ? theme.palette.common.black
              : theme.palette.text.secondary,
          mb: 2,
        }}
      >
        Advanced filtering mode is active
      </Typography>
      <Grid container spacing={3}>
        <Grid size={{ xs: 12, sm: 6 }}>
          <Box
            sx={{
              p: 3,
              borderRadius: 3,
              background: theme.palette.background.default,
              mb: 2,
              border: `1px solid ${theme.palette.divider}`,
            }}
          >
            <Box display="flex" alignItems="center" mb={1}>
              <AccessTimeIcon
                sx={{
                  color:
                    theme.palette.mode === "dark"
                      ? theme.palette.common.white
                      : theme.palette.common.black,
                  mr: 1,
                }}
              />
              <Typography
                variant="h6"
                sx={{ color: theme.palette.text.primary, fontSize: 17 }}
              >
                Uptime
              </Typography>
              <Box flexGrow={1} />
              <Typography
                variant="h6"
                sx={{ color: theme.palette.primary.main, fontSize: 17 }}
              >
                {uptime[0]}% - {uptime[1]}%
              </Typography>
            </Box>
            <Slider
              value={uptime}
              onChange={(_, v) => setUptime(v as [number, number])}
              valueLabelDisplay="off"
              min={0}
              max={100}
              marks={marksPercent}
              sx={{
                color: green,
                height: 8,
                "& .MuiSlider-thumb": {
                  width: 24,
                  height: 24,
                  backgroundColor: green,
                },
              }}
            />
          </Box>
        </Grid>
        <Grid size={{ xs: 12, sm: 6 }}>
          <Box
            sx={{
              p: 3,
              borderRadius: 3,
              background: theme.palette.background.default,
              mb: 2,
              border: `1px solid ${theme.palette.divider}`,
            }}
          >
            <Box display="flex" alignItems="center" mb={1}>
              <PieChartIcon
                sx={{
                  color:
                    theme.palette.mode === "dark"
                      ? theme.palette.common.white
                      : theme.palette.common.black,
                  mr: 1,
                }}
              />
              <Typography
                variant="h6"
                sx={{ color: theme.palette.text.primary, fontSize: 17 }}
              >
                Saturation
              </Typography>
              <Box flexGrow={1} />
              <Typography
                variant="h6"
                sx={{ color: theme.palette.primary.main, fontSize: 17 }}
              >
                {saturation[0]}% - {saturation[1]}%
              </Typography>
            </Box>
            <Slider
              value={saturation}
              onChange={(_, v) => setSaturation(v as [number, number])}
              valueLabelDisplay="off"
              min={0}
              max={maxSaturation}
              marks={marksSaturation}
              sx={{
                color: green,
                height: 8,
                "& .MuiSlider-thumb": {
                  width: 24,
                  height: 24,
                  backgroundColor: green,
                },
              }}
            />
          </Box>
        </Grid>
        <Grid size={{ xs: 12, sm: 6 }}>
          <Box
            sx={{
              p: 3,
              borderRadius: 3,
              background: theme.palette.background.default,
              mb: 2,
              border: `1px solid ${theme.palette.divider}`,
            }}
          >
            <Box display="flex" alignItems="center" mb={1}>
              <PercentIcon
                sx={{
                  color:
                    theme.palette.mode === "dark"
                      ? theme.palette.common.white
                      : theme.palette.common.black,
                  mr: 1,
                }}
              />
              <Typography
                variant="h6"
                sx={{ color: theme.palette.text.primary, fontSize: 17 }}
              >
                Profit Margin
              </Typography>
              <Box flexGrow={1} />
              <Typography
                variant="h6"
                sx={{ color: theme.palette.primary.main, fontSize: 17 }}
              >
                {profitMargin[0]}% - {profitMargin[1]}%
              </Typography>
            </Box>
            <Slider
              value={profitMargin}
              onChange={(_, v) => setProfitMargin(v as [number, number])}
              valueLabelDisplay="off"
              min={0}
              max={100}
              marks={marksPercent}
              sx={{
                color: green,
                height: 8,
                "& .MuiSlider-thumb": {
                  width: 24,
                  height: 24,
                  backgroundColor: green,
                },
              }}
            />
          </Box>
        </Grid>
      </Grid>
    </Box>
  );

  if (onlyPanel) return panel;

  return (
    <Box sx={{ width: "100%" }}>
      <Button
        variant="outlined"
        color="inherit"
        startIcon={
          <FilterAltIcon
            sx={{
              color:
                theme.palette.mode === "light"
                  ? `${theme.palette.common.black} !important`
                  : `${theme.palette.common.white} !important`,
            }}
          />
        }
        onClick={() => setOpen && setOpen(!open)}
        sx={{
          borderRadius: 3,
          px: 4,
          py: 1.5,
          color:
            theme.palette.mode === "light"
              ? `${theme.palette.common.black} !important`
              : `${theme.palette.common.white} !important`,
          borderColor:
            theme.palette.mode === "light"
              ? theme.palette.grey[400]
              : theme.palette.common.white,
          background: "none",
          fontWeight: 500,
          fontSize: 20,
          "&:hover, &:focus": {
            background:
              theme.palette.mode === "light"
                ? "rgba(0,0,0,0.04)"
                : "rgba(255,255,255,0.05)",
            borderColor:
              theme.palette.mode === "light"
                ? theme.palette.grey[400]
                : theme.palette.common.white,
          },
        }}
      >
        Advanced Filters
      </Button>
      <Collapse in={open} timeout="auto" unmountOnExit>
        {panel}
      </Collapse>
    </Box>
  );
}
