import { labGrotesque, labGrotesqueMono } from "../fonts";

export const headings = {
  display: {
    fontFamily: labGrotesque?.style?.fontFamily,
    fontWeight: 400,
    fontSize: 208,
    lineHeight: 1,
    textTransform: "uppercase" as const,
    "@media (max-width: 768px)": {
      fontSize: 68,
    },
  },
  h1: {
    fontSize: 40,
    lineHeight: 1.2,
    fontWeight: 400,
    textTransform: "uppercase" as const,
    "@media (max-width: 768px)": {
      fontSize: 24,
    },
  },
  h2: {
    fontSize: 32,
    lineHeight: "38px",
    fontWeight: 400,
    textTransform: "uppercase" as const,
    "@media (max-width: 768px)": {
      fontSize: 24,
    },
  },
  h3: {
    fontFamily: labGrotesqueMono?.style?.fontFamily,
    fontSize: 24,
    lineHeight: "29px",
    letterSpacing: "5%",
    textTransform: "uppercase" as const,
    "@media (max-width: 768px)": {
      fontSize: 18,
    },
  },
  h4: {
    fontFamily: labGrotesqueMono?.style?.fontFamily,
    fontSize: 20,
    lineHeight: "24px",
    letterSpacing: "5%",
    textTransform: "uppercase" as const,
    "@media (max-width: 768px)": {
      fontSize: 18,
    },
  },
  h5: {
    fontFamily: labGrotesqueMono?.style?.fontFamily,
    fontSize: 16,
    lineHeight: "24px",
    letterSpacing: "5%",
    textTransform: "uppercase" as const,
    "@media (max-width: 768px)": {
      fontSize: 18,
    },
  },
  h6: {
    fontFamily: labGrotesqueMono?.style?.fontFamily,
    fontSize: 14,
    lineHeight: "24px",
    letterSpacing: "5%",
    textTransform: "uppercase" as const,
    "@media (max-width: 768px)": {
      fontSize: 18,
    },
  },
};

export const subtitles = {
  subtitle1: {
    fontFamily: labGrotesqueMono?.style?.fontFamily,
    textTransform: "uppercase" as const,
    fontWeight: 400,
    size: 16,
    lineHeight: "19px",
    letterSpacing: "5%",
  },
  subtitle2: {
    fontFamily: labGrotesqueMono?.style?.fontFamily,
    textTransform: "uppercase" as const,
    fontWeight: 400,
    fontSize: 14,
    lineHeight: "17px",
    letterSpacing: "5%",
  },
  subtitle3: {
    fontFamily: labGrotesqueMono?.style?.fontFamily,
    textTransform: "uppercase" as const,
    fontWeight: 400,
    fontSize: 12,
    lineHeight: "14.4px",
    letterSpacing: "5%",
  },
};

export const body = {
  body1: {
    fontSize: "24px",
    lineHeight: "33.6px",
    fontWeight: 400,
    "@media (max-width: 768px)": {
      fontSize: 18,
    },
  },
  body2: {
    fontSize: "20px",
    lineHeight: "28px",
    fontWeight: 300,
    "@media (max-width: 768px)": {
      fontSize: 18,
    },
  },
  body3: {
    fontSize: "16px",
    lineHeight: "22.4px",
    fontWeight: 300,
  },
  body4: {
    fontSize: "14px",
    lineHeight: 1.4,
    fontWeight: 300,
  },
  body5: {
    fontSize: "12px",
    lineHeight: "16.8px",
    fontWeight: 300,
  },
};
