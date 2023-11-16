import { useContext, useEffect, useState } from "react";
import Layout from "../components/Layout";
import { Location } from "../lib/types";
import { invoke } from "@tauri-apps/api";
import toast from "react-hot-toast";
import Spinner from "../components/Spinner";
import LocationList from "../components/LocationList";
import LocationContext, {
  LocationContextInterface,
} from "../context/LocationContext";
import VpnStatusContext, {
  VpnStatusContextInterface,
} from "../context/VpnStatusContext";
import { MdKeyboardArrowLeft } from "react-icons/md";
import { useNavigate } from "react-router-dom";
import Navbar from "../components/Navbar";

type Props = {};

const Locations = (props: Props) => {
  const navigate = useNavigate();
  const { loadingLocations, locations } = useContext(
    LocationContext
  ) as LocationContextInterface;

  const { vpnStatus } = useContext(
    VpnStatusContext
  ) as VpnStatusContextInterface;

  const listEnabled: boolean = (() => {
    if (vpnStatus === undefined) {
      return true;
    }

    switch (vpnStatus.type) {
      case "Accepted":
      case "Connecting":
      case "Connected":
      case "Disconnecting":
      case "ServerReady":
      case "ServerCreated":
        return false;

      default:
        break;
    }

    return true;
  })();

  let [search, setSearch] = useState("");

  const filteredLocations = () => {
    let lowerSearch = search.toLowerCase();
    if (search.length > 0) {
      const filtered = locations.filter((location) => {
        return (
          location.city.toLowerCase().includes(lowerSearch) ||
          location.country.toLowerCase().includes(lowerSearch) ||
          location.country_code.toLowerCase().includes(lowerSearch) ||
          (location.state &&
            location.state.toLowerCase().includes(lowerSearch)) ||
          (location.state_code &&
            location.state_code.toLowerCase().includes(lowerSearch))
        );
      });
      return filtered;
    } else {
      return locations;
    }
  };

  const filtered = filteredLocations();

  return (
    <Layout activeLocation={true}>
      <div>
        <Navbar header="Select location" />

        <div className="flex flex-col mx-2 items-center">
          <div className="input-group px-1">
            <button className="btn btn-square no-animation hover:cursor-auto">
              <svg
                xmlns="http://www.w3.org/2000/svg"
                className="h-6 w-6"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth="2"
                  d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
                />
              </svg>
            </button>
            <input
              type="text"
              autoCorrect="off"
              autoCapitalize="none"
              autoFocus={true}
              placeholder="Search location"
              className="input input-bordered focus:ring-1 focus:outline-none hover:ring-1 w-full font-semibold"
              onChange={(e) => setSearch(e.target.value)}
            />
          </div>
          {loadingLocations ? <Spinner className="w-20 h-20" /> : <></>}
        </div>
        {!loadingLocations && locations.length > 0 && filtered.length == 0 ? (
          <div className="mx-2 my-2">
            <div className="alert">
              <div>
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  className="stroke-current flex-shrink-0 h-6 w-6"
                  fill="none"
                  viewBox="0 0 24 24"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth="2"
                    d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
                  />
                </svg>
                <span className="font-bold">
                  No location matching "
                  {search.length > 10
                    ? search.slice(0, 10).concat("..")
                    : search}
                  "
                </span>
              </div>
            </div>
          </div>
        ) : (
          <></>
        )}
        <div className="mx-2">
          <LocationList enabled={listEnabled} locations={filtered} />
        </div>
      </div>
    </Layout>
  );
};

export default Locations;
