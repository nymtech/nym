// import * as React from 'react';
// import { Theme } from '@emotion/react';
// import { PaletteMode } from '@mui/material';
// import { createTheme } from '@mui/material/styles';
// import { ExplorerContext, ExplorerProvider } from './context/main';
// const { mode }: any = React.useContext(ExplorerContext);

// export const theme = createTheme({
//   palette: {
//     mode,
//   },
// });
// const nymPalette = {
//   // DARK THEME
//   primary: {
//     main: '#F2F2F2', // white, font, text, main text color
//   },
//   // LIGHT THEME
//   secondary: {
//     main: '#fff', // white, font, text, main text color
//   },
//   background: {
//     main: '#242C3D',
//   },
// };

// const { mode }: any = React.useContext(ExplorerContext);

// export const theme = createTheme({
//   palette: {
//     mode,
//   },
// });

// A custom theme for this app
// export const theme = createTheme({
//   typography: {
//     fontFamily:
//       'open sans, sans-serif, -apple-system, BlinkMacSystemFont, "Segoe UI", "Roboto", "Oxygen", "Ubuntu", "Helvetica Neue", Arial, sans-serif, "Apple Color Emoji", "Segoe UI Emoji", "Segoe UI Symbol"',
//     fontSize: 14,
//     fontWeightBold: 600,
//   },
//   palette: {
//     primary: {
//       main: nymPalette.primary.main,
//     },
//     secondary: {
//       main: nymPalette.secondary.main,
//     },
//   },
//   shape: {
//     borderRadius: 24,
//   },
//   transitions: {
//     duration: {
//       shortest: 150,
//       shorter: 200,
//       short: 250,
//       standard: 300,
//       complex: 375,
//       enteringScreen: 225,
//       leavingScreen: 195,
//     },
//     easing: {
//       easeIn: 'cubic-bezier(0.4, 0, 1, 1)',
//     },
//   },
//   components: {
//     // MuiIconButton: {
//     //   styleOverrides: {
//     //     root: {
//     //       color: nymPalette.primary.light,
//     //     },
//     //   },
//     // },
//     MuiLink: {
//       styleOverrides: {
//         root: {
//           textDecoration: 'none',
//         },
//       },
//     },
//     MuiSvgIcon: {
//       styleOverrides: {
//         root: {
//           color: nymPalette.primary.main,
//         },
//       },
//     },
//     // MuiDrawer: {
//     //   styleOverrides: {
//     //     root: {
//     //       color: nymPalette.primary.light,
//     //       backgroundColor: nymPalette.primary.light,
//     //     },
//     //     paper: {
//     //       color: nymPalette.primary.light,
//     //       backgroundColor: nymPalette.primary.light,
//     //     },
//     //   },
//     // },
//     // MuiToolbar: {
//     //   styleOverrides: {
//     //     root: {
//     //       backgroundColor: nymPalette.primary.darker,
//     //       color: nymPalette.primary.main,
//     //     },
//     //   },
//     // },
//   },
// });
