import type { Config } from 'tailwindcss';
import defaultTheme from 'tailwindcss/defaultTheme';
import headlessui from '@headlessui/tailwindcss';

export default {
  content: ['./index.html', './src/**/*.{js,ts,jsx,tsx}'],
  theme: {
    colors: {
      transparent: 'transparent',
      current: 'currentColor',
      'baltic-sea': {
        // [D] bg for top-bar nav
        // [D] bg for network modes
        jaguar: '#2B2831',
        // [L] status-line title text + icon
        // [L] connection timer text
        // [L] "Connecting" status text
        // [L] main titles text
        // [L] network mode title text + icon
        // [L] node location select text value + icon + label
        // [D] button text
        DEFAULT: '#1C1B1F', // [D] main bg
      },
      // [L] main bg
      'blanc-nacre': '#F2F4F6',
      // [DL] primary accent
      melon: '#FB6E4E',
      // [DL] secondary accent
      cornflower: '#7075FF',
      // [DL] error status text
      teaberry: '#E33B5A',
      comet: '#625B71',
      // [DL] "Connected" status text
      'vert-menthe': '#2BC761',
      // [D] main titles text
      // [D] connection timer text
      // [D] "Connecting" status text
      // [L] bg for top-bar nav
      // [L] bg for network modes
      // [L] button text
      white: '#FFF',
      'flawed-white': '#FFFBFE',
      black: '#000',
      mercury: {
        // [D] status-line title text + icon
        // [D] network mode title text + icon
        // [D] node location select text value + icon + label
        pinkish: '#E6E1E5',
        DEFAULT: '#E1EFE7',
        // [D] network mode desc text
        // [D] "Connection time"
        // [D] main status desc text
        mist: '#938F99',
      },
      // [DL] "Disconnected" status text
      'coal-mine': { dark: '#56545A', light: '#A4A4A4' },
      // [L] "Connection time"
      // [L] main status desc text
      'dim-gray': '#696571',
      // [L] network mode desc text
      // [L] node location select outline
      // [L] connection status bg (combined with 10% opacity)
      'cement-feet': '#79747E',
      // [D] node location select outline
      'gun-powder': '#49454F',
      // [D] top-bar icon
      'laughing-jack': '#CAC4D0',
      // [L] button bg in disabled state
      'wind-chime': '#DEDEE1',
      // [D] connection status bg (combined with 15% opacity)
      oil: '#313033',
      // [DL] "Connected" status bg (combined with 10% opacity)
      'vert-prasin': '#47C45D',
    },
    extend: {
      fontFamily: {
        sans: ['Lato', ...defaultTheme.fontFamily.sans],
        icon: [
          'Material Symbols Outlined',
          {
            fontVariationSettings: '"opsz" 24;',
          },
        ],
      },
    },
  },
  plugins: [headlessui],
  // Toggling dark mode manually
  darkMode: 'class',
} satisfies Config;
