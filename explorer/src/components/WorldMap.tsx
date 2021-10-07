import React from 'react';
import { scaleLinear, ScaleLinear } from 'd3-scale';
import { ComposableMap, Geographies, Geography, ZoomableGroup, Marker } from 'react-simple-maps';
import ReactTooltip from 'react-tooltip';
import { Box } from '@mui/material';
import { MainContext } from '../context/main';
import { CountryDataResponse, ApiState } from 'src/typeDefs/explorer-api';
import { MAP_TOPOJSON } from 'src/api/constants';
import { Domain } from 'domain';

type MapProps = {
  title?: string,
  userLocation?: [number, number],
  countryData?: ApiState<CountryDataResponse>,
  loading: boolean
}

// type Range = ScaleContinuousNumeric<number, number>

export const WorldMap: React.FC<MapProps> = ({ title, countryData, userLocation, loading }) => {

  const [colorScale, setColorScale] = React.useState<() => ScaleLinear<string, string>>();
  const [hasNoContent, setHasNoContent] = React.useState<boolean>(true);
  const [tooltipContent, setTooltipContent] = React.useState<string | null>(null);
  const { mode } = React.useContext(MainContext);

  React.useEffect(() => {
    if (userLocation || countryData) {
      setHasNoContent(false);
    }
    if (countryData?.data) {
      const heighestNumberOfNodes = Math.max(...countryData.data.map(country => country.nodes));
      const cs = scaleLinear<string, string>()
        .domain([0, heighestNumberOfNodes])
        .range(mode === 'dark' ? ['#FFD278', '#F20041'] : ['orange', 'red']);
      setColorScale(() => cs);
    }
  }, [countryData, userLocation])

  return (
    <>
      <Box>
        <ComposableMap
          data-tip=""
          style={{
            backgroundColor:
              mode === 'dark'
                ? 'rgba(50, 60, 81, 1)'
                : 'rgba(241, 234, 234, 1)',
            WebkitFilter: hasNoContent ? 'blur(5px)' : null,
            filter: hasNoContent ? 'blur(5px)' : null,
          }}
          projectionConfig={{
            rotate: [-10, 0, 0],
            scale: 180,
          }}
        >
          <ZoomableGroup>

            <Geographies geography={MAP_TOPOJSON}>
              {({ geographies }: any) =>
                (colorScale || userLocation) && geographies.map((geo: any) => {
                  // @ts-ignore
                  const d = countryData && countryData.data && countryData.data.find((s) => s.ISO3 === geo.properties.ISO_A3)
                  return (
                    <Geography
                      key={geo.rsmKey}
                      geography={geo}
                      // @ts-ignore
                      fill={d ? colorScale(d.nodes) : '#FFFFFF'}
                      stroke="#EAEAEC"
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
                        !userLocation && countryData && {
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
      </Box>
    </>
  );
};
