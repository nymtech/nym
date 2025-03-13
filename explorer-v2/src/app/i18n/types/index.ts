import type { languages } from "../settings";

export type Languages = (typeof languages)[number];

// They don't belong to any specific page, that's why I defined them here, but open to suggestions.
export type GeneralTranslations = {
  alert: string;
  copyToClipboard: {
    copy: string;
    copied: string;
    copiedTimeLimitation: string;
  };
};
