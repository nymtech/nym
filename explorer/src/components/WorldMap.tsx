import React from 'react';
import { scaleLinear } from 'd3-scale';
import { ComposableMap, Geographies, Geography, ZoomableGroup, Marker } from 'react-simple-maps';
import ReactTooltip from 'react-tooltip';
import { Box, Typography } from '@mui/material';
import { MainContext } from '../context/main';
import { ContentCard } from './ContentCard';
import { CountryDataResponse, ApiState } from 'src/typeDefs/explorer-api';


const markers = [
  {
    markerOffset: -30,
    name: "Buenos Aires",
    coordinates: [-58.3816, -34.6037]
  },
  { markerOffset: 15, name: "La Paz", coordinates: [-68.1193, -16.4897] },
  { markerOffset: 15, name: "Brasilia", coordinates: [-47.8825, -15.7942] },
  { markerOffset: 15, name: "Santiago", coordinates: [-70.6693, -33.4489] },
  { markerOffset: 15, name: "Bogota", coordinates: [-74.0721, 4.711] },
  { markerOffset: 15, name: "Quito", coordinates: [-78.4678, -0.1807] },
  { markerOffset: -30, name: "Georgetown", coordinates: [-58.1551, 6.8013] },
  { markerOffset: -30, name: "Asuncion", coordinates: [-57.5759, -25.2637] },
  { markerOffset: 15, name: "Paramaribo", coordinates: [-55.2038, 5.852] },
  { markerOffset: 15, name: "Montevideo", coordinates: [-56.1645, -34.9011] },
  { markerOffset: 15, name: "Caracas", coordinates: [-66.9036, 10.4806] },
  { markerOffset: 15, name: "Lima", coordinates: [-77.0428, -12.0464] }
];

const geoUrl =
  'https://raw.githubusercontent.com/zcreativelabs/react-simple-maps/master/topojson-maps/world-110m.json';

type MapProps = {
  title?: string,
  userLocation?: [number, number],
  countryData?: ApiState<CountryDataResponse>,
}

export const WorldMap: React.FC<MapProps> = ({ title, countryData, userLocation }) => {

  const [tooltipContent, setTooltipContent] = React.useState<string | null>(null);
  const { mode } = React.useContext(MainContext);

  const colorScale: any = scaleLinear()
    .domain([0, 1200])
    // @ts-ignore
    .range(mode === 'dark' ? ['#ffedea', '#ff5233'] : ['orange', 'red']);

  return (
    <ContentCard title={title}>
      <Box>
        <ComposableMap
          data-tip=""
          style={{
            backgroundColor:
              mode === 'dark'
                ? 'rgba(50, 60, 81, 1)'
                : 'rgba(241, 234, 234, 1)',
          }}
          projectionConfig={{
            rotate: [-10, 0, 0],
            scale: 180,
            // scale: size === 'lg' ? 187 : 100,
          }}
        >
          <ZoomableGroup>

            <Geographies geography={geoUrl}>
              {({ geographies }: any) =>
                geographies.map((geo: any) => {
                  // @ts-ignore
                  const d = countryData && countryData.data && countryData.data.find((s) => s.ISO3 === geo.properties.ISO_A3)
                  return (
                    <Geography
                      key={geo.rsmKey}
                      geography={geo}
                      // @ts-ignore
                      fill={d ? colorScale(d.nodes) : '#F5F4F6'}
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
                        !userLocation && {
                          hover: {
                            fill: 'black',
                            outline: 'white',
                          },
                        }}
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
    </ContentCard>
  );
};
