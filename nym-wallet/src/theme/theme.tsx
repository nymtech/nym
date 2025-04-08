import { PaletteMode, alpha, Theme } from '@mui/material';
import {
  PaletteOptions,
  ThemeOptions,
  createTheme,
  Components,
} from '@mui/material/styles';

/**
 * The Nym palette.
 * 
 * IMPORTANT: do not export this constant, always use the MUI `useTheme` hook to get the correct
 * colours for dark/light mode.
 */
const nymPalette = {
  /** emphasises important elements */
  highlight: 'rgb(20, 231, 111)',
  success: 'rgb(20, 231, 111)',
  info: '#60D7EF',
  red: '#E33B5A',
  fee: '#967FF0',
  background: {
    light: '#242B2D',
    dark: '#1C1B1F',
  },
  text: {
    light: '#E6E1E5',
    dark: '#FFFFFF',
    muted: '#938F99',
    grey: '#79747E',
  },
  linkHover: 'rgb(20, 231, 111)',
  border: {
    menu: '#49454F',
  },
};

const darkMode = {
  mode: 'dark' as const,
  background: {
    main: '#242B2D',
    paper: '#32373D',
    warn: '#F97316',
    grey: '#3A373F',
    greyStroke: '#49454F',
    // New additions for depth and layering
    elevated: '#383E42',
    subtle: '#2A3134',
  },
  text: {
    main: '#FFFFFF',
    muted: '#938F99',
    warn: '#F97316',
    contrast: '#242B2D',
    grey: '#79747E',
    blue: '#60D7EF',
    subdued: '#B8B5BD',
  },
  topNav: {
    background: '#1C1B1F',
  },
  nav: {
    background: '#32373D',
  },
  hover: {
    background: '#3A373F',
  },
  modal: {
    border: '#49454F',
  },
  chart: {
    grey: '#49454F',
  },
  // New additions for modern look
  gradients: {
    primary: 'linear-gradient(45deg, rgba(20, 231, 111, 0.9), rgba(20, 231, 111, 0.7))',
    subtle: 'linear-gradient(180deg, rgba(50, 55, 61, 0.8), rgba(36, 43, 45, 0.8))',
  },
  shadows: {
    light: '0 2px 8px rgba(0, 0, 0, 0.15)',
    medium: '0 4px 12px rgba(0, 0, 0, 0.2)',
    strong: '0 8px 24px rgba(0, 0, 0, 0.3)',
  },
};

const lightMode = {
  mode: 'light' as const,
  background: {
    main: '#FFFFFF',
    paper: '#F4F6F8',
    warn: '#F97316',
    grey: '#E2E8EC',
    greyStroke: '#8DA3B1',
    elevated: '#FFFFFF',
    subtle: '#F9FAFB',
  },
  text: {
    main: '#1C1B1F',
    muted: '#79747E',
    warn: '#F97316',
    contrast: '#FFFFFF',
    grey: '#696571',
    blue: '#60D7EF',
    subdued: '#908E95',
  },
  topNav: {
    background: '#FFFFFF',
  },
  nav: {
    background: '#F4F6F8',
  },
  hover: {
    background: '#E2E8EC',
  },
  modal: {
    border: 'transparent',
  },
  chart: {
    grey: '#79747E',
  },
  // New modern additions
  gradients: {
    primary: 'linear-gradient(45deg, rgba(20, 231, 111, 0.9), rgba(20, 231, 111, 0.7))',
    subtle: 'linear-gradient(180deg, rgba(255, 255, 255, 1), rgba(244, 246, 248, 0.8))',
  },
  shadows: {
    light: '0 2px 8px rgba(0, 0, 0, 0.06)',
    medium: '0 4px 12px rgba(0, 0, 0, 0.08)',
    strong: '0 8px 24px rgba(0, 0, 0, 0.12)',
  },
};

// Type definition for variant to fix TS errors
type NymVariant = typeof darkMode | typeof lightMode;

const nymWalletPalette = (variant: NymVariant) => ({
  nymWallet: variant,
});

const variantToMUIPalette = (variant: NymVariant): PaletteOptions => ({
  text: {
    primary: variant.text.main,
    secondary: variant.text.subdued, // Using the new subdued color
    disabled: variant.text.grey,
  },
  primary: {
    main: nymPalette.highlight,
    light: alpha(nymPalette.highlight, 0.8),
    dark: alpha(nymPalette.highlight, 1.2),
    contrastText: variant.text.contrast,
  },
  secondary: {
    main: variant.text.blue,
    contrastText: variant.text.contrast,
  },
  success: {
    main: nymPalette.success,
    light: alpha(nymPalette.success, 0.8),
    dark: alpha(nymPalette.success, 1.2),
  },
  info: {
    main: nymPalette.info,
    light: alpha(nymPalette.info, 0.8),
    dark: alpha(nymPalette.info, 1.2),
  },
  error: {
    main: nymPalette.red,
    light: alpha(nymPalette.red, 0.8),
    dark: alpha(nymPalette.red, 1.2),
  },
  warning: {
    main: variant.background.warn,
    contrastText: '#FFFFFF',
  },
  background: {
    default: variant.background.main,
    paper: variant.background.paper,
  },
  divider: variant.background.greyStroke,
});

const createLightModePalette = (): PaletteOptions => ({
  nym: {
    ...nymPalette,
    ...nymWalletPalette(lightMode),
  },
  ...variantToMUIPalette(lightMode),
});

const createDarkModePalette = (): PaletteOptions => ({
  nym: {
    ...nymPalette,
    ...nymWalletPalette(darkMode),
  },
  ...variantToMUIPalette(darkMode),
});

// Define component overrides with proper types
const getComponentOverrides = (mode: PaletteMode): Components<Theme> => {
  return {
    MuiCssBaseline: {
      styleOverrides: {
        body: {
          scrollbarColor: `${mode === 'dark' ? darkMode.background.greyStroke : lightMode.background.greyStroke
            } transparent`,
          '&::-webkit-scrollbar, & *::-webkit-scrollbar': {
            width: '8px',
            height: '8px',
            backgroundColor: 'transparent',
          },
          '&::-webkit-scrollbar-thumb, & *::-webkit-scrollbar-thumb': {
            borderRadius: 8,
            backgroundColor: mode === 'dark' ? darkMode.background.greyStroke : lightMode.background.greyStroke,
            minHeight: 24,
          },
          '&::-webkit-scrollbar-corner, & *::-webkit-scrollbar-corner': {
            backgroundColor: 'transparent',
          },
        },
      },
    },
    MuiButton: {
      styleOverrides: {
        root: {
          fontSize: 15,
          padding: '8px 20px',
          borderRadius: 10,
          boxShadow: 'none',
          fontWeight: 600,
          transition: '0.2s all ease-in-out',
          '&:hover': {
            transform: 'translateY(-1px)',
            boxShadow: mode === 'dark' ? darkMode.shadows.medium : lightMode.shadows.medium,
          },
        },
        contained: {
          '&:hover': {
            boxShadow: mode === 'dark' ? darkMode.shadows.medium : lightMode.shadows.medium,
          },
        },
        // Use different approach for overriding MUI styles that have type issues
        containedPrimary: ({ theme }) => ({
          background: mode === 'dark' ? darkMode.gradients.primary : lightMode.gradients.primary,
          '&:hover': {
            background: mode === 'dark' ? darkMode.gradients.primary : lightMode.gradients.primary,
          },
        }),
        outlined: {
          borderWidth: '2px',
          '&:hover': {
            borderWidth: '2px',
          },
        },
        sizeLarge: {
          height: 52,
          fontSize: 16,
          padding: '10px 24px',
        },
        sizeSmall: {
          height: 36,
          fontSize: 14,
          padding: '6px 16px',
        },
      },
    },
    MuiCard: {
      styleOverrides: {
        // Use callback format for proper typing
        root: ({ theme }) => ({
          borderRadius: 16,
          boxShadow: mode === 'dark' ? darkMode.shadows.light : lightMode.shadows.light,
          transition: 'transform 0.3s, box-shadow 0.3s',
          '&:hover': {
            transform: 'translateY(-4px)',
            boxShadow: mode === 'dark' ? darkMode.shadows.medium : lightMode.shadows.medium,
          },
        }),
      },
    },
    MuiCardContent: {
      styleOverrides: {
        root: {
          padding: '24px',
          '&:last-child': {
            paddingBottom: '24px',
          },
        },
      },
    },
    MuiTextField: {
      styleOverrides: {
        root: {
          '& .MuiOutlinedInput-root': {
            borderRadius: 10,
            '& fieldset': {
              borderWidth: '2px',
            },
            '&:hover fieldset': {
              borderWidth: '2px',
            },
            '&.Mui-focused fieldset': {
              borderWidth: '2px',
            },
          },
        },
      },
    },
    MuiOutlinedInput: {
      styleOverrides: {
        root: {
          borderRadius: 10,
          '& fieldset': {
            borderWidth: '2px',
            transition: 'border-color 0.2s ease-in-out',
          },
          '&:hover .MuiOutlinedInput-notchedOutline': {
            borderWidth: '2px',
          },
          '&.Mui-focused .MuiOutlinedInput-notchedOutline': {
            borderWidth: '2px',
          },
        },
        input: {
          padding: '14px 16px',
        },
      },
    },
    MuiSwitch: {
      styleOverrides: {
        root: ({ theme }) => ({
          width: 62,
          height: 34,
          padding: 0,
          margin: theme.spacing(1),
          overflow: 'visible',
          '& .MuiSwitch-switchBase': {
            padding: 3,
            border: '2px solid transparent',
            borderRadius: '50%',
            transition: theme.transitions.create(['transform', 'background-color'], {
              duration: 500,
            }),
            '&.Mui-checked': {
              transform: 'translateX(28px)',
              '& + .MuiSwitch-track': {
                backgroundColor: nymPalette.highlight,
                opacity: 1,
                border: 0,
              },
              '& .MuiSwitch-thumb': {
                backgroundColor: '#fff',
                boxShadow: '0px 0px 8px rgba(0, 0, 0, 0.2)',
              },
              '&.Mui-disabled + .MuiSwitch-track': {
                opacity: 0.5,
              },
            },
            '&.Mui-disabled': {
              opacity: 0.5,
            },
            '&.Mui-disabled .MuiSwitch-thumb': {
              opacity: 0.8,
            },
          },
          '& .MuiSwitch-thumb': {
            boxSizing: 'border-box',
            width: 22,
            height: 22,
            backgroundColor: mode === 'dark' ? '#fff' : '#383838',
            borderRadius: '50%',
            transition: theme.transitions.create(['width', 'transform', 'background-color'], {
              duration: 500,
            }),
            boxShadow: '0px 2px 4px rgba(0, 0, 0, 0.2)',
          },
          '& .MuiSwitch-track': {
            borderRadius: 17,
            backgroundColor: mode === 'dark'
              ? 'rgba(255, 255, 255, 0.1)'
              : 'rgba(0, 0, 0, 0.1)',
            border: `1px solid ${mode === 'dark' ? 'rgba(255, 255, 255, 0.2)' : 'rgba(0, 0, 0, 0.1)'}`,
            opacity: 1,
            transition: theme.transitions.create(['background-color', 'border'], {
              duration: 500,
            }),
            '&:before, &:after': {
              content: '""',
              position: 'absolute',
              top: '50%',
              transform: 'translateY(-50%)',
              width: 16,
              height: 16,
            },
            '&:before': {
              backgroundImage: `url('data:image/svg+xml;utf8,<svg xmlns="http://www.w3.org/2000/svg" height="16" width="16" viewBox="0 0 24 24"><path fill="${encodeURIComponent(
                mode === 'dark' ? '#fff' : '#383838',
              )}" d="M20 8.69V4h-4.69L12 .69 8.69 4H4v4.69L.69 12 4 15.31V20h4.69L12 23.31 15.31 20H20v-4.69L23.31 12 20 8.69zm-2 5.79V18h-3.52L12 20.48 9.52 18H6v-3.52L3.52 12 6 9.52V6h3.52L12 3.52 14.48 6H18v3.52L20.48 12 18 14.48zM12 6.5c-3.03 0-5.5 2.47-5.5 5.5s2.47 5.5 5.5 5.5 5.5-2.47 5.5-5.5-2.47-5.5-5.5-5.5zm0 9c-1.93 0-3.5-1.57-3.5-3.5s1.57-3.5 3.5-3.5 3.5 1.57 3.5 3.5-1.57 3.5-3.5 3.5z"/></svg>')`,
              left: 8,
              opacity: mode === 'dark' ? 0 : 0.7,
              transition: 'opacity 0.3s',
            },
            '&:after': {
              backgroundImage: `url('data:image/svg+xml;utf8,<svg xmlns="http://www.w3.org/2000/svg" height="16" width="16" viewBox="0 0 24 24"><path fill="${encodeURIComponent(
                '#fff',
              )}" d="M12,7c-2.76,0-5,2.24-5,5s2.24,5,5,5s5-2.24,5-5S14.76,7,12,7L12,7z M2,13l2,0c0.55,0,1-0.45,1-1s-0.45-1-1-1l-2,0 c-0.55,0-1,0.45-1,1S1.45,13,2,13z M20,13l2,0c0.55,0,1-0.45,1-1s-0.45-1-1-1l-2,0c-0.55,0-1,0.45-1,1S19.45,13,20,13z M11,2v2 c0,0.55,0.45,1,1,1s1-0.45,1-1V2c0-0.55-0.45-1-1-1S11,1.45,11,2z M11,20v2c0,0.55,0.45,1,1,1s1-0.45,1-1v-2c0-0.55-0.45-1-1-1 S11,19.45,11,20z M5.99,4.58c-0.39-0.39-1.03-0.39-1.41,0c-0.39,0.39-0.39,1.03,0,1.41l1.06,1.06c0.39,0.39,1.03,0.39,1.41,0 s0.39-1.03,0-1.41L5.99,4.58z M18.36,16.95c-0.39-0.39-1.03-0.39-1.41,0c-0.39,0.39-0.39,1.03,0,1.41l1.06,1.06 c0.39,0.39,1.03,0.39,1.41,0c0.39-0.39,0.39-1.03,0-1.41L18.36,16.95z M19.42,5.99c0.39-0.39,0.39-1.03,0-1.41 c-0.39-0.39-1.03-0.39-1.41,0l-1.06,1.06c-0.39,0.39-0.39,1.03,0,1.41s1.03,0.39,1.41,0L19.42,5.99z M7.05,18.36 c0.39-0.39,0.39-1.03,0-1.41c-0.39-0.39-1.03-0.39-1.41,0l-1.06,1.06c-0.39,0.39-0.39,1.03,0,1.41s1.03,0.39,1.41,0L7.05,18.36z"/></svg>')`,
              right: 8,
              opacity: mode === 'dark' ? 0.7 : 0,
              transition: 'opacity 0.3s',
            },
          },
        }),
      },
    },
    MuiTable: {
      styleOverrides: {
        root: {
          borderCollapse: 'separate',
          borderSpacing: '0 8px',
        },
      },
    },
    MuiTableRow: {
      styleOverrides: {
        root: {
          borderRadius: 12,
          '&.MuiTableRow-hover:hover': {
            backgroundColor: mode === 'dark'
              ? alpha(darkMode.hover.background, 0.5)
              : alpha(lightMode.hover.background, 0.5),
          },
        },
      },
    },
    MuiTableCell: {
      styleOverrides: {
        root: {
          padding: '12px 16px',
          lineHeight: 1.5,
          borderBottom: 'none',
          '&:first-of-type': {
            borderTopLeftRadius: 12,
            borderBottomLeftRadius: 12,
          },
          '&:last-of-type': {
            borderTopRightRadius: 12,
            borderBottomRightRadius: 12,
          },
        },
        // Using function format for styleOverrides to avoid type errors
        head: ({ theme }) => ({
          fontWeight: 600,
          color: mode === 'dark' ? darkMode.text.subdued : lightMode.text.subdued,
          backgroundColor: 'transparent',
        }),
        body: ({ theme }) => ({
          backgroundColor: mode === 'dark' ? darkMode.background.paper : lightMode.background.paper,
        }),
      },
    },
    MuiToolbar: {
      styleOverrides: {
        root: {
          minWidth: 0,
          padding: '8px 16px',
          '@media (min-width: 0px)': {
            minHeight: 'fit-content',
          },
        },
      },
    },
    MuiDialog: {
      styleOverrides: {
        // Using function format for styleOverrides to avoid type errors
        paper: ({ theme }) => ({
          borderRadius: 16,
          boxShadow: mode === 'dark' ? darkMode.shadows.strong : lightMode.shadows.strong,
          backgroundImage: mode === 'dark' ? darkMode.gradients.subtle : lightMode.gradients.subtle,
          backgroundSize: 'cover',
        }),
      },
    },
    MuiDialogTitle: {
      styleOverrides: {
        root: {
          fontSize: '1.5rem',
          fontWeight: 700,
          padding: '24px 24px 16px',
        },
      },
    },
    MuiDialogContent: {
      styleOverrides: {
        root: {
          padding: '16px 24px 24px',
        },
      },
    },
    MuiChip: {
      styleOverrides: {
        root: {
          borderRadius: 8,
          fontWeight: 500,
          '&.MuiChip-colorPrimary': {
            background: mode === 'dark'
              ? alpha(nymPalette.highlight, 0.15)
              : alpha(nymPalette.highlight, 0.12),
            color: nymPalette.highlight,
          },
        },
        label: {
          padding: '0 12px',
        },
        sizeMedium: {
          height: 32,
        },
        sizeSmall: {
          height: 26,
        },
      },
    },
    MuiLink: {
      defaultProps: {
        underline: 'none',
      },
      styleOverrides: {
        root: {
          fontWeight: 500,
          transition: 'color 0.2s',
          '&:hover': {
            color: nymPalette.linkHover,
          },
        },
      },
    },
    MuiDivider: {
      styleOverrides: {
        root: {
          opacity: 0.6,
        },
      },
    },
    MuiTooltip: {
      styleOverrides: {
        // Using function format for styleOverrides to avoid type errors
        tooltip: ({ theme }) => ({
          borderRadius: 8,
          padding: '8px 16px',
          fontSize: '0.75rem',
          fontWeight: 500,
          boxShadow: mode === 'dark' ? darkMode.shadows.medium : lightMode.shadows.medium,
        }),
      },
    },
    MuiLinearProgress: {
      styleOverrides: {
        // Using function format for styleOverrides to avoid type errors
        root: ({ theme }) => ({
          borderRadius: 6,
          height: 8,
          backgroundColor: mode === 'dark'
            ? alpha(darkMode.background.greyStroke, 0.5)
            : alpha(lightMode.background.greyStroke, 0.5),
        }),
        bar: {
          borderRadius: 6,
        },
        colorPrimary: {
          '& .MuiLinearProgress-bar': {
            backgroundImage: mode === 'dark'
              ? darkMode.gradients.primary
              : lightMode.gradients.primary,
          },
        },
      },
    },
    MuiStepIcon: {
      styleOverrides: {
        root: {
          '&.Mui-completed': {
            color: nymPalette.success,
          },
          '&.Mui-active': {
            color: nymPalette.highlight,
          },
        },
      },
    },
    MuiSelect: {
      defaultProps: {
        MenuProps: {
          PaperProps: {
            sx: {
              borderRadius: 2,
              boxShadow: mode === 'dark' ? darkMode.shadows.medium : lightMode.shadows.medium,
              mt: 1,
              '&& .Mui-selected': {
                color: nymPalette.highlight,
                backgroundColor: mode === 'dark'
                  ? alpha(darkMode.background.main, 0.9)
                  : alpha(lightMode.background.main, 0.9),
              },
              '&& .Mui-selected:hover': {
                backgroundColor: mode === 'dark'
                  ? alpha(nymPalette.highlight, 0.08)
                  : alpha(nymPalette.highlight, 0.08),
              },
              '& .MuiMenuItem-root': {
                borderRadius: 1,
                margin: '2px 6px',
                padding: '8px 12px',
                '&:hover': {
                  backgroundColor: mode === 'dark'
                    ? alpha(darkMode.hover.background, 0.7)
                    : alpha(lightMode.hover.background, 0.7),
                },
              },
            },
          },
        },
      },
    },
    MuiMenu: {
      styleOverrides: {
        list: ({ theme }) => ({
          backgroundColor: mode === 'dark' ? darkMode.background.main : lightMode.background.main,
          border: `1px solid ${mode === 'dark'
            ? darkMode.background.greyStroke
            : lightMode.background.greyStroke}`,
          borderRadius: '12px',
          padding: '8px 0',
        }),
      },
    },
    MuiMenuItem: {
      styleOverrides: {
        root: {
          borderRadius: 8,
          margin: '2px 8px',
          padding: '8px 12px',
          '&:hover': {
            backgroundColor: mode === 'dark'
              ? alpha(darkMode.hover.background, 0.7)
              : alpha(lightMode.hover.background, 0.7),
          },
        },
      },
    },
    MuiAppBar: {
      styleOverrides: {
        // Using function format for styleOverrides to avoid type errors
        root: ({ theme }) => ({
          boxShadow: mode === 'dark'
            ? '0 4px 12px rgba(0, 0, 0, 0.1)'
            : '0 4px 12px rgba(0, 0, 0, 0.05)',
          backgroundImage: 'none',
        }),
      },
    },
    MuiPaper: {
      styleOverrides: {
        root: {
          backgroundImage: 'none',
        },
        // Using function format for styleOverrides to avoid type errors
        elevation1: ({ theme }) => ({
          boxShadow: mode === 'dark' ? darkMode.shadows.light : lightMode.shadows.light,
        }),
        elevation4: ({ theme }) => ({
          boxShadow: mode === 'dark' ? darkMode.shadows.medium : lightMode.shadows.medium,
        }),
      },
    },
  };
};

export const getDesignTokens = (mode: PaletteMode): ThemeOptions => {
  const { palette } = createTheme({
    palette: {
      mode,
      ...(mode === 'light' ? createLightModePalette() : createDarkModePalette()),
    },
  });

  return {
    typography: {
      fontFamily: ['Inter', 'Lato', 'sans-serif', 'BlinkMacSystemFont', 'Roboto', 'Oxygen', 'Ubuntu', 'Helvetica Neue'].join(
        ',',
      ),
      fontSize: 14,
      fontWeightLight: 300,
      fontWeightRegular: 400,
      fontWeightMedium: 500,
      fontWeightBold: 700,
      h1: {
        fontSize: '2.5rem',
        fontWeight: 700,
        lineHeight: 1.2,
        letterSpacing: '-0.01em',
      },
      h2: {
        fontSize: '2rem',
        fontWeight: 700,
        lineHeight: 1.3,
        letterSpacing: '-0.005em',
      },
      h3: {
        fontSize: '1.75rem',
        fontWeight: 600,
        lineHeight: 1.3,
      },
      h4: {
        fontSize: '1.5rem',
        fontWeight: 600,
        lineHeight: 1.4,
      },
      h5: {
        fontSize: '1.25rem',
        fontWeight: 600,
        lineHeight: 1.4,
      },
      h6: {
        fontSize: '1rem',
        fontWeight: 600,
        lineHeight: 1.5,
      },
      subtitle1: {
        fontSize: '1rem',
        fontWeight: 500,
        lineHeight: 1.5,
        letterSpacing: '0.005em',
      },
      subtitle2: {
        fontSize: '0.875rem',
        fontWeight: 500,
        lineHeight: 1.5,
        letterSpacing: '0.005em',
      },
      body1: {
        fontSize: '1rem',
        lineHeight: 1.5,
        letterSpacing: '0.001em',
      },
      body2: {
        fontSize: '0.875rem',
        lineHeight: 1.6,
        letterSpacing: '0.001em',
      },
      button: {
        textTransform: 'none',
        fontWeight: 600,
        letterSpacing: '0.01em',
      },
      caption: {
        fontSize: '0.75rem',
        lineHeight: 1.5,
        letterSpacing: '0.02em',
      },
      overline: {
        fontSize: '0.75rem',
        fontWeight: 600,
        lineHeight: 1.5,
        letterSpacing: '0.05em',
        textTransform: 'uppercase',
      },
    },
    shape: {
      borderRadius: 12,
    },
    transitions: {
      duration: {
        shortest: 120,
        shorter: 180,
        short: 220,
        standard: 280,
        complex: 350,
        enteringScreen: 225,
        leavingScreen: 195,
      },
      easing: {
        easeInOut: 'cubic-bezier(0.4, 0, 0.2, 1)',
        easeOut: 'cubic-bezier(0.0, 0, 0.2, 1)',
        easeIn: 'cubic-bezier(0.4, 0, 1, 1)',
        sharp: 'cubic-bezier(0.4, 0, 0.6, 1)',
      },
    },
    components: getComponentOverrides(mode),
    palette,
  };
};