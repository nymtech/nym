import type { Config } from 'tailwindcss';

export default {
  content: ['./index.html', './src/**/*.{js,ts,jsx,tsx}'],
  theme: {
    colors: {
      transparent: 'transparent',
      current: 'currentColor',
      'baltic-sea': {
        light: '#2B2831',
        DEFAULT: '#1C1B1F',
      },
      oil: '#1F1F22',
      quartzite: '#202C25',
      melon: '#FB6E4E',
      cornflower: '#7075FF',
    },
    extend: {},
  },
  plugins: [],
  // Toggling dark mode manually
  darkMode: 'class',
} satisfies Config;
