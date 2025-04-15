"use client";

import * as React from "react";
import { scaleLinear } from "d3-scale";
import {
  ComposableMap,
  Geographies,
  Geography,
  ZoomableGroup,
} from "react-simple-maps";
import { Tooltip } from "react-tooltip";
import "react-tooltip/dist/react-tooltip.css";
import { Skeleton, Stack, Typography, IconButton } from "@mui/material";
import { useTheme } from "@mui/material/styles";
import { CountryDataResponse } from "../../app/api/types";
import MAP_TOPOJSON from "../../assets/world-110m.json";
import { useQuery } from "@tanstack/react-query";
import { fetchWorldMapCountries } from "@/app/api";
import ExplorerCard from "../cards/ExplorerCard";
import AddIcon from "@mui/icons-material/Add";
import RemoveIcon from "@mui/icons-material/Remove";
import RestartAltIcon from "@mui/icons-material/RestartAlt";

export const WorldMap = ({}): JSX.Element => {
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
    staleTime: 10 * 60 * 1000, // 10 minutes
    refetchOnWindowFocus: false,
    refetchOnReconnect: false,
    refetchOnMount: false,
  });

  const [tooltipContent, setTooltipContent] = React.useState<string | null>(
    null
  );

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
  }, [
    countries,
    theme.palette.mode,
    theme.palette.pine,
    theme.palette.accent,
    isDarkMode,
  ]);

  if (isLoadingCountries) {
    return (
      <ExplorerCard label="Nym Nodes in the world">
        <Stack gap={1}>
          <Skeleton variant="text" />
          <Skeleton variant="text" height={238} />
        </Stack>
      </ExplorerCard>
    );
  }

  if (isCountriesError) {
    return (
      <ExplorerCard label="Nym Nodes in the world">
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
      </ExplorerCard>
    );
  }

  return (
    // <ExplorerCard label="Nym Nodes in the world">
    <>
      <div
        style={{
          position: "relative",
          width: "100%",
          overflow: "hidden",
          margin: "0 auto",
        }}
      >
        {/* <div style={{ width: "90%", margin: "0 auto" }}> */}
        <ComposableMap
          {...({} as any)}
          data-tip=""
          style={{
            backgroundColor: isDarkMode ? "#000000" : theme.palette.pine[25],
            width: "100%",
            height: "auto",
          }}
          viewBox="50, 80, 800, 400"
          projection="geoEqualEarth"
          projectionConfig={{
            scale: 150,
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
                      key={`${geo.properties?.ISO_A3 || ""}-${geo.id}-${geo.properties?.NAME_LONG || ""}`}
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
                            }
                          : undefined,
                      }}
                    />
                  );
                })
              }
            </Geographies>
          </ZoomableGroup>
        </ComposableMap>
        {/* </div> */}

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
        content={tooltipContent || ""}
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
          zIndex: 1000,
        }}
      />
      {/* // </ExplorerCard> */}
    </>
  );
};
