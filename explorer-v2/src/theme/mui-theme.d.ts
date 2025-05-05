import type { CSSProperties } from "react";

// Define allowed values for textTransform
type TextTransform = "none" | "capitalize" | "uppercase" | "lowercase";

// Define a custom type for typography variants that includes textTransform
type CustomTypography = CSSProperties & { textTransform?: TextTransform };

declare module "@mui/material/styles" {
  interface Palette {
    accent: Palette["primary"];
    backgroundColor: Palette["background"];
    medium: Palette["primary"];
    light: Palette["primary"];
    gray: Palette["primary"];
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
    accent?: PaletteOptions["primary"];
    backgroundColor?: PaletteOptions["background"];
    medium?: PaletteOptions["primary"];
    light?: PaletteOptions["primary"];
    gray?: PaletteOptions["primary"];
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

  // Apply CustomTypography to TypographyVariants
  interface TypographyVariants {
    display: CustomTypography;
    h1: CustomTypography;
    h2: CustomTypography;
    h3: CustomTypography;
    h4: CustomTypography;
    h5: CustomTypography;
    h6: CustomTypography;
    subtitle1: CustomTypography;
    subtitle2: CustomTypography;
    subtitle3: CustomTypography;
    body3?: CustomTypography;
    body4?: CustomTypography;
    body5?: CustomTypography;
    prose?: CustomTypography;
  }

  // Apply CustomTypography to TypographyVariantsOptions
  interface TypographyVariantsOptions {
    display?: CustomTypography;
    h1?: CustomTypography;
    h2?: CustomTypography;
    h3?: CustomTypography;
    h4?: CustomTypography;
    h5?: CustomTypography;
    h6?: CustomTypography;
    subtitle1?: CustomTypography;
    subtitle2?: CustomTypography;
    subtitle3?: CustomTypography;
    body3?: CustomTypography;
    body4?: CustomTypography;
    body5?: CustomTypography;
    prose?: CustomTypography;
  }
}

declare module "@mui/material/Typography" {
  interface TypographyPropsVariantOverrides {
    display: true;
    subtitle3: true;
    body3: true;
    body4: true;
    body5: true;
    prose: true;
  }
}
