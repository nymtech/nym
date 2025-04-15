declare module "react-simple-maps" {
  import type { FunctionComponent, ReactNode } from "react";
  import type { Feature, Geometry, GeoJsonProperties } from "geojson";

  export interface ComposableMapProps {
    children?: ReactNode;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    [key: string]: any;
  }

  export interface ZoomableGroupProps {
    children?: ReactNode;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    [key: string]: any;
  }

  export interface GeographiesProps {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    geography: any;
    children?: (props: {
      geographies: Feature<Geometry, GeoJsonProperties>[];
    }) => ReactNode;
  }

  export interface GeographyProps {
    geography: Feature<Geometry, GeoJsonProperties>;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    [key: string]: any;
  }

  export const ComposableMap: FunctionComponent<ComposableMapProps>;
  export const ZoomableGroup: FunctionComponent<ZoomableGroupProps>;
  export const Geographies: FunctionComponent<GeographiesProps>;
  export const Geography: FunctionComponent<GeographyProps>;
}
