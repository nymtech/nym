import React, { ReactNode } from "react";
import { Location } from "../lib/types";
import Country from "./Country";

type Props = {
  locations?: Location[];
  enabled: boolean;
};

function LocationList({ locations, enabled }: Props) {
  let countries = locations?.reduce<Map<string, Location[]>>((acc, current) => {
    if (acc.has(current.country_code)) {
      acc.get(current.country_code)?.push(current);
    } else {
      acc.set(current.country_code, [current]);
    }
    return acc;
  }, new Map<string, Location[]>());

  let countryComponents: ReactNode[] = [];

  countries?.forEach((locations, countryCode) => {
    countryComponents.push(
      <Country
        country_code={countryCode}
        locations={locations}
        enabled={enabled}
      />
    );
  });

  return (
    <div className="flex flex-col my-2 w-full">{...countryComponents}</div>
  );
}

export default LocationList;
