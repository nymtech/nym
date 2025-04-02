// get full country name
export const countryName = (countryCode: string | null) => {
  if (countryCode) {
    const regionNames = new Intl.DisplayNames(["en"], { type: "region" });

    return regionNames.of(countryCode);
  }
  return countryCode;
};
