declare module "react-simple-maps" {
  import { FunctionComponent, ReactNode } from "react";
  import { Feature, Geometry, GeoJsonProperties } from "geojson";

  export interface ComposableMapProps {
    children?: ReactNode;
    [key: string]: any;
  }

  export interface ZoomableGroupProps {
    children?: ReactNode;
    [key: string]: any;
  }

  export interface GeographiesProps {
    geography: any;
    children?: (props: {
      geographies: Feature<Geometry, GeoJsonProperties>[];
    }) => ReactNode;
  }

  export interface GeographyProps {
    geography: Feature<Geometry, GeoJsonProperties>;
    [key: string]: any;
  }

  export const ComposableMap: FunctionComponent<ComposableMapProps>;
  export const ZoomableGroup: FunctionComponent<ZoomableGroupProps>;
  export const Geographies: FunctionComponent<GeographiesProps>;
  export const Geography: FunctionComponent<GeographyProps>;
}
