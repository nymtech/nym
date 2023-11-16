import React from "react";
import { Location } from "../lib/types";

import City from "./City";

type Props = {
  country_code: string;
  locations: Location[];
  enabled: boolean;
};

function Country({ country_code, locations, enabled }: Props) {
  return (
    <ul className="menu bg-base-100 p-1 rounded-box">
      <li className="menu-title">
        <div>{locations[0].country}</div>
      </li>

      {locations.map((loc) => {
        return (
          <li key={loc.code} className={enabled ? "" : "disabled"}>
            <City location={loc} enabled={enabled} />
          </li>
        );
      })}
    </ul>
  );
}

export default Country;
