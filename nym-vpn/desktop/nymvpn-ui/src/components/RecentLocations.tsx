import React from "react";
import City from "./City";
import { Location } from "../lib/types";

type Props = {
  locations: Location[];
  disabled: boolean;
};

const RecentLocations = ({ locations, disabled }: Props) => {
  const cities = locations.map((location) => {
    return (
      <li key={location.code} className={disabled ? "disabled" : ""}>
        <City location={location} key={location.code} enabled={!disabled} />
      </li>
    );
  });

  return (
    <div className="card  bg-base-300 max-h-40 overflow-y-auto">
      <ul className="menu my-2 p-2 rounded-box">
        <li className="menu-title">
          <div>Recent Locations</div>
        </li>
        {cities}
      </ul>
    </div>
  );
};

export default RecentLocations;
