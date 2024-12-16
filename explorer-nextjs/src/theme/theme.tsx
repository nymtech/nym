import { labGrotesque } from "@/fonts";
import { colours } from "@/theme/colours";
import { body, headings, subtitles } from "@/theme/typography";
import { type ThemeOptions, createTheme } from "@mui/material/styles";
const lightMode = {
  mode: "light" as const,
  accent: colours.green[500],
  primary: colours.pine[950],
  background: colours.gray[200],
  medium: colours.haze[200],
  light: colours.base.white,
};

const darkMode = {
  mode: "dark" as const,
  accent: colours.green[500],
  primary: colours.base.white,
  background: colours.pine[950],
  medium: colours.pine[800],
  light: colours.pine[200],
};

// createDesignTokens function to generate design tokens based on mode
const createDesignTokens = (mode: "light" | "dark") => {
  const palette = mode === "light" ? lightMode : darkMode;

  return {
    palette: {
      mode: palette.mode,
      primary: {
        main: palette.primary,
      },
      background: {
        default: palette.background,
        main: palette.background,
        paper: palette.light,
      },
      secondary: {
        main: palette.accent,
      },
      text: {
        primary: palette.primary,
        secondary: palette.medium,
      },
      success: {
        main: colours.alert.success,
      },
      warning: {
        main: colours.alert.warning,
      },
      error: {
        main: colours.alert.error,
      },
      medium: {
        main: palette.medium,
      },
      accent: {
        main: palette.accent,
      },
      light: {
        main: palette.light,
      },
      base: {
        white: colours.base.white,
        black: colours.base.black,
        transparent: colours.base.transparent,
      },
      grey: colours.gray,
      pine: colours.pine,
    },
    typography: {
      fontFamily: labGrotesque?.style?.fontFamily,
      weight: 300,
      ...headings,
      ...subtitles,
      ...body,
      prose: {
        ...body.body2,
        "& * > *": {
          marginBlockEnd: "30px",
        },
        "& h1": {
          ...headings.h1,
        },
        "& h2": {
          ...headings.h2,
        },
        "& h3": {
          ...headings.h3,
        },
        "& h4": {
          ...headings.h4,
        },
        "& h5": {
          ...headings.h5,
        },
        "& h6": {
          ...headings.h6,
        },
        "& p": {
          ...body.body2,
        },
        "& a": {
          color: palette.primary,
          textDecoration: "underline",
        },
        "& ul, & ol": {
          listStyleType: "disc",
          paddingLeft: "1.5rem",
          margin: "1rem 0",
        },
        "& li": {
          marginBottom: "0.5rem",
        },
        "& blockquote": {
          borderLeft: `4px solid ${palette.primary}`,
          paddingLeft: "1rem",
          margin: "1rem 0",
        },
        "& hr": {
          border: 0,
          height: 1,
          borderTop: `1px dashed ${palette.primary}`,
        },
        "& table": {
          width: "100%",
          borderCollapse: "collapse",
          borderSpacing: 0,
          "& th, & td": {
            borderBottom: `1px solid ${palette.background}`,
            paddingBlock: "0.5rem",
            textAlign: "left",
          },
          "& tr:last-of-type td": {
            borderBottom: "none",
          },
          "& th": {
            backgroundColor: palette.light,
            fontWeight: 700,
            color: palette.background,
          },
        },
      },
    },
  };
};

// Function to create MUI theme with additional options
const getTheme = (mode: "light" | "dark"): ThemeOptions => {
  const designTokens = createDesignTokens(mode);

  return {
    ...designTokens,
    components: {
      // Add component customizations here if needed
      MuiButtonBase: {
        defaultProps: {
          disableRipple: true,
          disableTouchRipple: true,
        },
      },
      MuiInputBase: {
        styleOverrides: {
          root: {
            fontSize: "16px",
            backgroundColor: colours.base.white,
          },
        },
      },
      MuiOutlinedInput: {
        styleOverrides: {
          root: {
            borderRadius: "32px",
          },
        },
      },
      MuiButton: {
        styleOverrides: {
          root: {
            ...designTokens.typography.subtitle1,
            boxShadow: "none",
            outline: "none",
            fontSize: "16px",
            borderRadius: "32px",
            padding: "16px 24px",
            variants: [
              {
                props: { size: "medium" },
                style: {
                  fontSize: "16px",
                  borderRadius: "32px",
                  padding: "16px 24px",
                },
              },
              {
                props: { size: "small" },
                style: {
                  fontSize: "14px",
                  padding: "8px 16px",
                },
              },
              {
                props: { variant: "outlined" },
                style: {
                  border: `1px dashed ${designTokens.palette.primary.main}`,
                  "&:hover": {
                    backgroundColor: "transparent",
                    borderStyle: "solid",
                  },
                  "&:active": {
                    borderStyle: "solid",
                    backgroundColor: designTokens.palette.primary.main,
                    color: designTokens.palette.background.main,
                    boxShadow: "none",
                  },
                  "&:focus-visible": {
                    boxShadow: `0 0 4px 0 ${designTokens.palette.primary.main}`,
                    borderStyle: "solid",
                  },
                  "&:disabled": {
                    border: `1px solid ${designTokens.palette.primary.main}`,
                    color: designTokens.palette.primary.main,
                    background: "transparent",
                    opacity: 0.5,
                  },
                },
              },
              {
                props: { variant: "contained" },
                style: {
                  backgroundColor: designTokens.palette.accent.main,
                  color: designTokens.palette.base.black,
                  border: `1px solid ${designTokens.palette.accent.main}`,
                  "&:hover": {
                    borderColor: designTokens.palette.primary.main,
                  },
                  "&:focus-visible": {
                    boxShadow: `0 0 4px 0 ${designTokens.palette.primary.main}`,
                    borderStyle: "solid",
                  },
                  "&:active": {
                    outline: `1px solid ${designTokens.palette.primary.main}`,
                  },
                  "&:disabled": {
                    backgroundColor: designTokens.palette.primary.main,
                    color: designTokens.palette.background.main,
                    borderColor: designTokens.palette.primary.main,
                    opacity: 0.5,
                  },
                },
              },
              {
                props: { variant: "text" },
                style: {
                  backgroundColor: "transparent",
                  color: designTokens.palette.primary.main,
                  "&:hover": {
                    backgroundColor: "transparent",
                    color: designTokens.palette.primary.main,
                    textDecoration: "underline",
                  },
                  "&:focus-visible": {
                    boxShadow: `0 0 4px 0 ${designTokens.palette.primary.main}`,
                    borderStyle: "solid",
                  },
                  "&:active": {
                    color: "black",
                    outline: "none",
                    borderColor: "transparent",
                  },
                  "&:disabled": {
                    color: designTokens.palette.primary.main,
                    opacity: 0.5,
                  },
                },
              },
            ],
          },
        },
      },
      // remove border radius from paper
      MuiPaper: {
        styleOverrides: {
          root: {
            borderRadius: 0,
          },
        },
      },
    },
  };
};

// Create light and dark themes
const lightTheme = createTheme(getTheme("light"));
const darkTheme = createTheme(getTheme("dark"));

export { lightTheme, darkTheme, getTheme };
