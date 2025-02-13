import { createInstance } from "i18next";
import resourcesToBackend from "i18next-resources-to-backend";
import { initReactI18next } from "react-i18next/initReactI18next";
import { getOptions } from "./settings";
import type { Languages } from "./types";

const initI18next = async ({ lng, ns }: { lng: Languages; ns?: string }) => {
  const i18nInstance = createInstance();
  await i18nInstance
    .use(initReactI18next)
    .use(
      resourcesToBackend(
        (language: string, namespace: string) =>
          import(`./locales/${language}/${namespace}.json`),
      ),
    )
    .init(getOptions(lng, ns));
  return i18nInstance;
};

export const useTranslation = async ({
  lng,
  ns,
  options,
}: {
  lng: Languages;
  ns?: string;
  options?: { keyPrefix: string };
}) => {
  const i18nextInstance = await initI18next({ lng, ns });
  return {
    t: i18nextInstance.getFixedT(
      lng,
      Array.isArray(ns) ? ns[0] : ns,
      options?.keyPrefix,
    ),
    i18n: i18nextInstance,
  };
};

export type { Languages, GeneralTranslations } from "./types";
