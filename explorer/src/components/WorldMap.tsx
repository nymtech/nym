import React from 'react';
import { geoEqualEarth, geoPath } from 'd3-geo';
import { feature } from 'topojson-client';
import { Feature, FeatureCollection, Geometry } from 'geojson';
import { Box, Grid, Typography } from '@mui/material';

const uuid = require('react-uuid');

const cx = 400;
const cy = 150;

export type WorldMapProps = {
  // text: string;
  // SVGIcon: React.FunctionComponent<any>;
  // url: string;
};

export const WorldMap: React.FC<WorldMapProps> = () => {
  const [geographies, setGeographies] = React.useState<
    [] | Array<Feature<Geometry | null>>
  >([]);

  const [scale, setScale] = React.useState<number>(200);
  const [windowResizing, setWindowResizing] = React.useState<boolean>(false);

  const projection = geoEqualEarth()
    .scale(scale)
    .translate([cx, cy])
    .rotate([0, 0]);

  const fetchGeographicData = (): void => {
    fetch(
      'https://raw.githubusercontent.com/d3/d3.github.com/master/world-110m.v1.json',
    ).then((response) => {
      if (response.status !== 200) {
        console.log('coordinates for the map have failed to load');
        return;
      }
      response.json().then((worldData) => {
        const mapFeatures: Array<Feature<Geometry | null>> = (
          feature(
            worldData,
            worldData.objects.countries,
          ) as unknown as FeatureCollection
        ).features;
        setGeographies(mapFeatures);
      });
    });
  };

  const renderMap = () => {
    console.log('render the map...');
  };

  React.useEffect(() => {
    fetchGeographicData();
    renderMap();
  }, []);

  // React.useEffect(() => {
  //   let timeout: any;
  //   const handleResize = () => {
  //     clearTimeout(timeout);

  //     setWindowResizing(true);

  //     timeout = setTimeout(() => {
  //       setWindowResizing(false);
  //     }, 200);
  //   };
  //   window.addEventListener('resize', handleResize);

  //   return () => window.removeEventListener('resize', handleResize);
  // }, []);

  // React.useEffect(() => {
  //   if (!windowResizing) {
  //     const n = window.innerWidth;
  //     if (n < 830) {
  //       console.log('tablet/mobile now');
  //       setScale(150);
  //     }
  //     if (n > 829 && n < 1300) {
  //       console.log('laptop now');
  //       setScale(200);
  //     }
  //     if (n > 1300) {
  //       console.log('Large Desktop now');
  //       setScale(240);
  //     }
  //   }
  // }, [windowResizing]);

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

        <svg
          width={scale * 4}
          height={scale * 4}
          viewBox={window.innerWidth < 600 ? '100 0 750 350' : '0 0 800 450'}
          data-testid="svg"
          style={{ border: '1px solid red' }}
        >
          <g>
            {(geographies as []).map((d, i) => (
              <path
                key={`path-${uuid()}`}
                d={geoPath().projection(projection)(d) as string}
                fill={`rgba(246, 195, 180,${
                  (1 / (geographies ? geographies.length : 0)) * i
                })`}
                stroke="grey"
                strokeWidth={0.5}
              />
            ))}
          </g>
        </svg>
      </Box>
    </Grid>
  );
};
