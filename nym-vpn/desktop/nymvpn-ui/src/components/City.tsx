import React, { useContext, useState } from "react";
import { Location } from "../lib/types";
import ReactCountryFlag from "react-country-flag";
import LocationContext, {
  LocationContextInterface,
} from "../context/LocationContext";
import { toast } from "react-hot-toast";

type Props = {
  location: Location;
  enabled: boolean;
};

function City({ location, enabled }: Props) {
  const { selected, setLocation } = useContext(
    LocationContext
  ) as LocationContextInterface;

  const onClick = () => {
    if (enabled === true) {
      setLocation(location);
    } else {
      toast.error("Cannot change location when vpn session is in progress");
    }
  };

  return (
    <div className="flex justify-between" onClick={onClick}>
      <div className="flex gap-2 items-center">
        <ReactCountryFlag
          className="rounded"
          svg
          countryCode={location.country_code}
          style={{
            width: "1.5em",
          }}
        />
        <div className="font-bold">{location.city}</div>
      </div>
      <input
        type="radio"
        className="radio"
        checked={(selected && selected.code === location.code) || false}
        onChange={() => {}}
        disabled={enabled ? false : true}
      />
    </div>
  );
}

export default City;
