import React from 'react';
// import { csv, json } from 'd3-fetch';
import { scaleLinear } from 'd3-scale';
import { ComposableMap, Geographies, Geography } from 'react-simple-maps';
import ReactTooltip from 'react-tooltip';
import { Box, Grid, Typography } from '@mui/material';
import { countriesData } from '../data/countriesData';

const geoUrl =
  'https://raw.githubusercontent.com/zcreativelabs/react-simple-maps/master/topojson-maps/world-110m.json';

export type WorldMapProps = {
  // text: string;
  // SVGIcon: React.FunctionComponent<any>;
  // url: string;
};

const colorScale: any = scaleLinear()
  .domain([0, 1200])
  // @ts-ignore
  .range(['#ffedea', '#ff5233']);

export const WorldMap: React.FC<WorldMapProps> = () => {
  const [tooltipContent, setTooltipContent] = React.useState<string>('');
  const [data, setData] = React.useState<Record<string, unknown>[]>([]);

  React.useEffect(() => {
    setData(countriesData);
  }, []);

  return (
    <Grid
      item
      xs={12}
      sx={{
        justifyContent: 'flex-start',
        padding: (theme) => theme.spacing(2),
        backgroundColor: (theme) => theme.palette.primary.dark,
      }}
    >
      <Box
        sx={{
          padding: (theme) => theme.spacing(3),
          backgroundColor: (theme) => theme.palette.primary.light,
        }}
      >
        <Typography
          sx={{
            color: (theme) => theme.palette.primary.main,
          }}
        >
          Distribution of nodes around the world
        </Typography>

        <ComposableMap
          data-tip=""
          style={{ backgroundColor: 'rgba(50, 60, 81, 1)', marginTop: 30 }}
          projectionConfig={{
            rotate: [-10, 0, 0],
            scale: 187,
          }}
        >
          {data.length > 0 && (
            <Geographies geography={geoUrl}>
              {({ geographies }: any) =>
                geographies.map((geo: any) => {
                  const d = data.find((s) => s.ISO3 === geo.properties.ISO_A3);
                  console.log('geo', geo);
                  console.log('d', d);
                  return (
                    <Geography
                      key={geo.rsmKey}
                      geography={geo}
                      // @ts-ignore
                      fill={d ? colorScale(d.nodes) : '#F5F4F6'}
                      onMouseEnter={() => {
                        const { NAME_LONG } = geo.properties;
                        setTooltipContent(
                          // @ts-ignore
                          `${NAME_LONG} | ${d?.nodes || 0}`,
                        );
                      }}
                      onMouseLeave={() => {
                        setTooltipContent('');
                      }}
                      style={{
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
          )}
        </ComposableMap>
        <ReactTooltip>{tooltipContent}</ReactTooltip>
      </Box>
    </Grid>
  );
};
