import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import main from './en/main.json';

const defaultNS = 'main';

i18n.use(initReactI18next).init({
  lng: 'en',
  debug: true,
  resources: {
    en: {
      main,
    },
  },
  defaultNS,

  interpolation: {
    escapeValue: false, // not needed for react as it escapes by default
  },
});

export default i18n;
