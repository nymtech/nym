import "@mui/material/styles";

declare module "@mui/material/styles" {
  interface Palette {
    nym: {
      highlight: string;
      networkExplorer: {
        map: {
          stroke: string;
          fills: string[];
        };
        background: {
          tertiary: string;
        };
      };
    };
    green: {
      500: string;
    };
    pine: {
      25: string;
      200: string;
      300: string;
      600: string;
      800: string;
      900: string;
      950: string;
    };
    base: {
      white: string;
      black: string;
      transparent: string;
    };
  }
  interface PaletteOptions {
    nym?: {
      highlight?: string;
      networkExplorer?: {
        map?: {
          stroke?: string;
          fills?: string[];
        };
        background?: {
          tertiary?: string;
        };
      };
    };
    green?: {
      500?: string;
    };
    pine?: {
      25?: string;
      200?: string;
      300?: string;
      600?: string;
      800?: string;
      900?: string;
      950?: string;
    };
    base?: {
      white?: string;
      black?: string;
      transparent?: string;
    };
  }
}
