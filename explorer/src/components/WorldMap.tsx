import React, { useEffect, useState } from 'react';
import { geoEqualEarth, geoPath } from 'd3-geo';
import { feature } from 'topojson-client';
import { Feature, FeatureCollection, Geometry } from 'geojson';
// import './styles.css';

const uuid = require('react-uuid');

const cx = 400;
const cy = 150;

const WorldMap: React.FunctionComponent = () => {
  const [geographies, setGeographies] = useState<
    [] | Array<Feature<Geometry | null>>
  >([]);

  const [scale, setScale] = useState<number>(200);
  const [windowResizing, setWindowResizing] = useState<boolean>(false);

  const projection = geoEqualEarth()
    .scale(scale)
    .translate([cx, cy])
    .rotate([0, 0]);

  const fetchGeographicData = (): void => {
    fetch('/data/world-110m.json').then((response) => {
      if (response.status !== 200) {
        console.log('Houston, we have a problem');
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

  useEffect(() => {
    fetchGeographicData();
    renderMap();
  }, []);

  useEffect(() => {
    let timeout: any;
    const handleResize = () => {
      clearTimeout(timeout);

      setWindowResizing(true);

      timeout = setTimeout(() => {
        setWindowResizing(false);
      }, 200);
    };
    window.addEventListener('resize', handleResize);

    return () => window.removeEventListener('resize', handleResize);
  }, []);

  useEffect(() => {
    if (!windowResizing) {
      const n = window.innerWidth;
      if (n < 830) {
        console.log('tablet/mobile now');
        setScale(150);
      }
      if (n > 829 && n < 1300) {
        console.log('laptop now');
        setScale(200);
      }
      if (n > 1300) {
        console.log('Large Desktop now');
        setScale(240);
      }
    }
  }, [windowResizing]);

  return (
    <div data-testid="worldMap__container" className="worldMap__container">
      <h1>Mixnodes Around the Globe</h1>
      <svg
        width={scale * 4}
        height={scale * 4}
        viewBox={window.innerWidth < 600 ? '100 0 750 350' : '0 0 800 450'}
        data-testid="svg"
      >
        <g>
          {(geographies as []).map((d, i) => (
            <path
              key={`path-${uuid()}`}
              d={geoPath().projection(projection)(d) as string}
              fill={`rgba(38,50,56,${
                (1 / (geographies ? geographies.length : 0)) * i
              })`}
              stroke="aliceblue"
              strokeWidth={0.5}
            />
          ))}
        </g>
      </svg>
    </div>
  );
};

export default WorldMap;
