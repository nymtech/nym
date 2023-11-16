import React, { useContext } from "react";
import { useNavigate } from "react-router-dom";
import ReactCountryFlag from "react-country-flag";
import { MdKeyboardArrowRight } from "react-icons/md";
import { IoReload } from "react-icons/io5";
import LocationContext, {
  LocationContextInterface,
} from "../context/LocationContext";
import Spinner from "./Spinner";
import { defaultLocation } from "../lib/util";

type Props = {
  enabled: boolean;
  waitingToConnect: boolean;
};

function LocationSelector({ enabled, waitingToConnect }: Props) {
  const navigate = useNavigate();
  const { loadingLocations, selected, locations, getLocations } = useContext(
    LocationContext
  ) as LocationContextInterface;

  // When locations are loading display spinner
  if (loadingLocations) {
    return <Spinner className="w-12 h-12" />;
  }

  const found = locations.find((value) => {
    return value.city.includes("Ashburn") || value.city.includes("Hillsboro");
  });

  const location = selected ? selected : defaultLocation(locations);

  // When no location is available let user reload locations
  if (!location) {
    return (
      <div className="tooltip tooltip-bottom" data-tip="Reload locations">
        <button
          disabled={false}
          onClick={() => {
            getLocations();
          }}
          className="btn btn-ghost btn-wide gap-1"
        >
          <div className="ml-2 text-error">No locations. Retry</div>
          <IoReload size="1.5em" />
        </button>
      </div>
    );
  }

  let displayLocation = location.city;

  if (location.country_code == "US" || location.city_code == "CA") {
    displayLocation = `${displayLocation}${
      location.state_code ? `, ${location.state_code}` : ""
    }`;
  }

  return (
    <button
      disabled={enabled ? false : true}
      onClick={() => {
        navigate("/locations");
      }}
      className={`btn btn-ghost btn-wide gap-1 ${
        waitingToConnect ? "loading" : ""
      }`}
    >
      <ReactCountryFlag
        className="rounded"
        svg
        countryCode={location.country_code}
        style={{
          width: "1.5em",
        }}
      />
      <div className="ml-2">{displayLocation}</div>
      <MdKeyboardArrowRight
        size="1.5em"
        className={waitingToConnect ? "hidden" : "block"}
      />
    </button>
  );
}

export default LocationSelector;
