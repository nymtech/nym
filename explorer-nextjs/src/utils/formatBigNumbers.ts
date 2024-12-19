const defaultOptions: Intl.NumberFormatOptions = {
  maximumFractionDigits: 2,
  notation: "compact",
  compactDisplay: "short",
};

const defaultLocale = "en-US";

export const formatBigNum = (
  num: number,
  locale = defaultLocale,
  opts = defaultOptions,
) => {
  return new Intl.NumberFormat(locale, opts || defaultOptions).format(num);
};
