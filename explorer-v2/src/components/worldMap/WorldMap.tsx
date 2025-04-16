"use client";

import { scaleLinear } from "d3-scale";
import * as React from "react";
import {
  ComposableMap,
  Geographies,
  Geography,
  ZoomableGroup,
} from "react-simple-maps";
import { Tooltip } from "react-tooltip";
import { fetchWorldMapCountries } from "@/app/api";
import AddIcon from "@mui/icons-material/Add";
import RemoveIcon from "@mui/icons-material/Remove";
import RestartAltIcon from "@mui/icons-material/RestartAlt";
import { IconButton, Skeleton, Stack, Typography } from "@mui/material";
import { useTheme } from "@mui/material/styles";
import { useQuery } from "@tanstack/react-query";
import type { CountryDataResponse } from "../../app/api/types";
import MAP_TOPOJSON from "../../assets/world-110m.json";
import ExplorerCard from "../cards/ExplorerCard";

export const WorldMap = (): JSX.Element => {
  const theme = useTheme();
  const isDarkMode = theme.palette.mode === "dark";
  const [position, setPosition] = React.useState<{
    coordinates: [number, number];
    zoom: number;
  }>({ coordinates: [0, 0], zoom: 1 });

  const {
    data: countries = [],
    isLoading: isLoadingCountries,
    isError: isCountriesError,
  } = useQuery({
    queryKey: ["nymNodesCountries"],
    queryFn: fetchWorldMapCountries,
    staleTime: 10 * 60 * 1000,
    refetchOnWindowFocus: false,
    refetchOnReconnect: false,
  });

  const [tooltipContent, setTooltipContent] = React.useState<string>("");

  React.useEffect(() => {
    const handleMouseLeave = () => setTooltipContent("");
    return () => {
      handleMouseLeave();
    };
  }, []);

  const colorScale = React.useMemo(() => {
    if (countries) {
      const heighestNumberOfNodes = Math.max(
        ...Object.values(countries).map((country) => country.nodes)
      );
      return scaleLinear<string, string>()
        .domain([
          0,
          1,
          heighestNumberOfNodes / 4,
          heighestNumberOfNodes / 2,
          heighestNumberOfNodes,
        ])
        .range(
          isDarkMode
            ? [
                theme.palette.pine[950],
                "#0F5A2E", // Dark green
                "#147A3D", // Medium green
                "#1A994C", // Light green
                theme.palette.accent.main,
              ]
            : [
                theme.palette.pine[300],
                "#0F5A2E", // Dark green
                "#147A3D", // Medium green
                "#1A994C", // Light green
                theme.palette.accent.main,
              ]
        )
        .unknown(isDarkMode ? theme.palette.pine[950] : theme.palette.pine[25]);
    }
    return () =>
      isDarkMode ? theme.palette.pine[950] : theme.palette.pine[25];
  }, [countries, theme.palette.pine, theme.palette.accent, isDarkMode]);

  if (isLoadingCountries) {
    return (
      <Stack gap={1} width="100%">
        <Skeleton variant="text" height={238} />
      </Stack>
    );
  }

  if (isCountriesError) {
    return (
      <Stack gap={1}>
        <Typography
          variant="h5"
          sx={{
            color: isDarkMode ? "base.white" : "pine.950",
            letterSpacing: 0.7,
          }}
        >
          Failed to load data
        </Typography>
        <Skeleton variant="text" height={238} />
      </Stack>
    );
  }

  console.log("countries", countries);

  return (
    <ExplorerCard label="Nym Nodes in the World" sx={{ width: "100%" }}>
      <div
        style={{
          position: "relative",
          width: "100%",
          overflow: "hidden",
          margin: "0 auto",
        }}
      >
        <ComposableMap
          data-tip=""
          style={{
            backgroundColor: isDarkMode ? "#000000" : theme.palette.pine[25],
            width: "100%",
            height: "auto",
          }}
          viewBox="0, 50, 800, 350"
          projection="geoMercator"
          projectionConfig={{
            scale: 100,
          }}
        >
          <ZoomableGroup
            center={position.coordinates}
            zoom={position.zoom}
            minZoom={1}
            maxZoom={8}
            translateExtent={[
              [-800, -400],
              [800, 400],
            ]}
            onMoveEnd={({
              coordinates,
              zoom,
            }: {
              coordinates: [number, number];
              zoom: number;
            }) => {
              setPosition({ coordinates, zoom });
            }}
            onClick={(e: React.MouseEvent) => {
              e.preventDefault();
              e.stopPropagation();
            }}
          >
            <Geographies geography={MAP_TOPOJSON}>
              {({ geographies }: { geographies: GeoJSON.Feature[] }) =>
                geographies.map((geo) => {
                  const d = Array.isArray(countries)
                    ? { nodes: 0 }
                    : (countries as CountryDataResponse)[
                        geo.properties?.ISO_A3 as string
                      ] || { nodes: 0 };
                  return (
                    <Geography
                      key={`${geo.properties?.ISO_A3 || ""}-${geo.id}-${
                        geo.properties?.NAME_LONG || ""
                      }`}
                      geography={geo}
                      fill={colorScale(d?.nodes || 0)}
                      stroke={
                        theme.palette.mode === "dark"
                          ? theme.palette.pine[800]
                          : theme.palette.pine[200]
                      }
                      strokeWidth={0.2}
                      data-tooltip-id="map-tooltip"
                      onMouseEnter={() => {
                        const { NAME_LONG } = geo.properties as {
                          NAME_LONG: string;
                        };
                        setTooltipContent(`${NAME_LONG} | ${d?.nodes || 0}`);
                      }}
                      onMouseLeave={() => {
                        setTooltipContent("");
                      }}
                      style={{
                        hover: countries
                          ? {
                              fill: theme.palette.accent.main,
                              outline: "white",
                              cursor: "pointer",
                            }
                          : undefined,
                        default: {
                          outline: "none",
                        },
                        pressed: {
                          outline: "none",
                        },
                      }}
                    />
                  );
                })
              }
            </Geographies>
          </ZoomableGroup>
        </ComposableMap>

        <div
          style={{
            position: "absolute",
            top: 10,
            left: 10,
            zIndex: 1000,
            display: "flex",
            flexDirection: "column",
            gap: "4px",
            backgroundColor: isDarkMode
              ? "rgba(0,0,0,0.5)"
              : "rgba(255,255,255,0.5)",
            padding: "4px",
            borderRadius: "4px",
          }}
        >
          <IconButton
            size="small"
            onClick={() =>
              setPosition((prev) => ({
                ...prev,
                zoom: Math.min(prev.zoom + 0.5, 8),
              }))
            }
            sx={{
              backgroundColor: isDarkMode
                ? "rgba(255,255,255,0.1)"
                : "rgba(0,0,0,0.1)",
              "&:hover": {
                backgroundColor: isDarkMode
                  ? "rgba(255,255,255,0.2)"
                  : "rgba(0,0,0,0.2)",
              },
            }}
          >
            <AddIcon fontSize="small" />
          </IconButton>
          <IconButton
            size="small"
            onClick={() =>
              setPosition((prev) => ({
                ...prev,
                zoom: Math.max(prev.zoom - 0.5, 1),
              }))
            }
            sx={{
              backgroundColor: isDarkMode
                ? "rgba(255,255,255,0.1)"
                : "rgba(0,0,0,0.1)",
              "&:hover": {
                backgroundColor: isDarkMode
                  ? "rgba(255,255,255,0.2)"
                  : "rgba(0,0,0,0.2)",
              },
            }}
          >
            <RemoveIcon fontSize="small" />
          </IconButton>
          <IconButton
            size="small"
            onClick={() => setPosition({ coordinates: [0, 0], zoom: 1 })}
            sx={{
              backgroundColor: isDarkMode
                ? "rgba(255,255,255,0.1)"
                : "rgba(0,0,0,0.1)",
              "&:hover": {
                backgroundColor: isDarkMode
                  ? "rgba(255,255,255,0.2)"
                  : "rgba(0,0,0,0.2)",
              },
            }}
          >
            <RestartAltIcon fontSize="small" />
          </IconButton>
        </div>
      </div>
      <Tooltip
        id="map-tooltip"
        content={tooltipContent}
        float={true}
        style={{
          fontSize: "12px",
          padding: "4px 8px",
          backgroundColor:
            theme.palette.mode === "dark"
              ? theme.palette.pine[800]
              : theme.palette.pine[200],
          color:
            theme.palette.mode === "dark"
              ? theme.palette.base.white
              : theme.palette.pine[950],
          borderRadius: "4px",
          zIndex: 9999,
        }}
      />
    </ExplorerCard>
  );
};
