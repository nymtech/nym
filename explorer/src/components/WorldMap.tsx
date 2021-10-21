import React from 'react';
import { scaleLinear, ScaleLinear } from 'd3-scale';
import {
  ComposableMap,
  Geographies,
  Geography,
  ZoomableGroup,
  Marker,
} from 'react-simple-maps';
import ReactTooltip from 'react-tooltip';
import { CountryDataResponse, ApiState } from 'src/typeDefs/explorer-api';
import { CircularProgress } from '@mui/material';
import { MainContext } from '../context/main';
import MAP_TOPOJSON from '../assets/world-110m.json';
import { palette } from '../index';

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
  const { mode } = React.useContext(MainContext);

  React.useEffect(() => {
    if (userLocation || countryData) {
      setHasNoContent(false);
    }
    if (countryData?.data) {
      const heighestNumberOfNodes = Math.max(
        ...countryData.data.map((country) => country.nodes),
      );
      const cs = scaleLinear<string, string>()
        .domain([0, heighestNumberOfNodes])
        .range([palette.mapLow, palette.mapHigh]);
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
          backgroundColor:
            mode === 'dark' ? palette.mapBgDark : palette.mapBgLight,
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
                // @ts-ignore
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
                    stroke="rgba(0, 0, 0, 0.3)"
                    strokeWidth={1}
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
                          fill: 'black',
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
