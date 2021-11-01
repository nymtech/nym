/* eslint-disable @typescript-eslint/ban-ts-comment */
import React from 'react';
import { scaleLinear, ScaleLinear } from 'd3-scale';
import {
  ComposableMap,
  Geographies,
  Geography,
  Marker,
  ZoomableGroup,
} from 'react-simple-maps';
import ReactTooltip from 'react-tooltip';
import { ApiState, CountryDataResponse } from 'src/typeDefs/explorer-api';
import { CircularProgress } from '@mui/material';
import { useTheme } from '@mui/material/styles';
import MAP_TOPOJSON from '../assets/world-110m.json';

type MapProps = {
  userLocation?: [number, number];
  countryData?: ApiState<CountryDataResponse>;
  loading: boolean;
};

export const WorldMap: React.FC<MapProps> = ({
  countryData,
  userLocation,
  loading,
}) => {
  const [colorScale, setColorScale] =
    React.useState<() => ScaleLinear<string, string>>();
  const [hasNoContent, setHasNoContent] = React.useState<boolean>(true);
  const [tooltipContent, setTooltipContent] = React.useState<string | null>(
    null,
  );
  const { palette } = useTheme();

  React.useEffect(() => {
    if (userLocation || countryData) {
      setHasNoContent(false);
    }
    if (countryData?.data) {
      const heighestNumberOfNodes = Math.max(
        ...countryData.data.map((country) => country.nodes),
      );
      const cs = scaleLinear<string, string>()
        .domain([
          0,
          heighestNumberOfNodes / 4,
          heighestNumberOfNodes / 2,
          heighestNumberOfNodes,
        ])
        .range(palette.nym.networkExplorer.map.fills);
      setColorScale(() => cs);
    }
  }, [countryData, userLocation]);

  if (loading) {
    return <CircularProgress />;
  }

  return (
    <>
      <ComposableMap
        data-tip=""
        style={{
          backgroundColor: palette.nym.networkExplorer.background.tertiary,
          WebkitFilter: hasNoContent ? 'blur(5px)' : null,
          filter: hasNoContent ? 'blur(5px)' : null,
          width: '100%',
          height: 'auto',
        }}
        width={800}
        height={350}
        projectionConfig={{
          scale: 120,
        }}
      >
        <ZoomableGroup>
          <Geographies geography={MAP_TOPOJSON}>
            {({ geographies }: any) =>
              (colorScale || userLocation) &&
              geographies.map((geo: any) => {
                const d =
                  countryData &&
                  countryData.data &&
                  countryData.data.find(
                    (s) => s.ISO3 === geo.properties.ISO_A3,
                  );
                return (
                  <Geography
                    key={geo.rsmKey}
                    geography={geo}
                    // @ts-ignore
                    fill={d ? colorScale(d.nodes) : '#FFFFFF'}
                    stroke="black"
                    strokeWidth={0.2}
                    onMouseEnter={() => {
                      const { NAME_LONG } = geo.properties;
                      if (!userLocation) {
                        setTooltipContent(
                          // @ts-ignore
                          `${NAME_LONG} | ${d?.nodes || 0}`,
                        );
                      }
                    }}
                    onMouseLeave={() => {
                      setTooltipContent('');
                    }}
                    style={
                      !userLocation &&
                      countryData && {
                        hover: {
                          fill: palette.nym.highlight,
                          outline: 'white',
                        },
                      }
                    }
                  />
                );
              })
            }
          </Geographies>

          {userLocation && (
            <Marker coordinates={userLocation}>
              <g
                fill="grey"
                stroke="#FF5533"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
                transform="translate(-12, -10)"
              >
                <circle cx="12" cy="10" r="5" />
              </g>
            </Marker>
          )}
        </ZoomableGroup>
      </ComposableMap>
      <ReactTooltip>{tooltipContent}</ReactTooltip>
    </>
  );
};

WorldMap.defaultProps = {
  userLocation: undefined,
  countryData: undefined,
};
