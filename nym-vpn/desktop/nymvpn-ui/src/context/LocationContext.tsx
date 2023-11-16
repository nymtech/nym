import { ReactNode, createContext, useState } from "react";
import { UiError, Location } from "../lib/types";
import { invoke } from "@tauri-apps/api";
import { handleError } from "../lib/util";
import { useNavigate } from "react-router-dom";

export interface LocationContextInterface {
  loadingLocations: boolean;
  locations: Location[];
  loadingRecent: boolean;
  recentLocations: Location[];
  selected?: Location;
  getLocations: () => void;
  getRecentLocations: () => void;
  setLocation: (location: Location) => void;
}

const LocationContext = createContext<LocationContextInterface | undefined>(
  undefined
);

export const LocationProvider = ({ children }: { children: ReactNode }) => {
  const navigate = useNavigate();
  const [loadingLocations, setLoadingLocations] = useState(false);
  const [locations, setLocations] = useState([] as Location[]);
  const [selected, setSelected] = useState<Location | undefined>(undefined);

  const [loadingRecent, setLoadingRecent] = useState(false);
  const [recentLocations, setRecentLocations] = useState([] as Location[]);

  const getLocations = () => {
    setLoadingLocations(true);
    const fetchLocations = async () => {
      try {
        const locations = (await invoke("locations")) as Location[];
        setLocations(locations);
      } catch (e) {
        const error = e as UiError;
        handleError(error, navigate);
      }
      setLoadingLocations(false);
    };

    fetchLocations();
  };

  const getRecentLocations = () => {
    setLoadingRecent(true);
    const fetchLocations = async () => {
      try {
        const locations = (await invoke("recent_locations")) as Location[];
        setRecentLocations(locations);
      } catch (e) {
        const error = e as UiError;
        handleError(error, navigate);
      }
      setLoadingRecent(false);
    };

    fetchLocations();
  };

  const setLocation = (location: Location) => {
    setSelected(location);
  };

  return (
    <LocationContext.Provider
      value={{
        loadingLocations,
        locations,
        loadingRecent,
        recentLocations,
        selected,
        getRecentLocations,
        getLocations,
        setLocation,
      }}
    >
      {children}
    </LocationContext.Provider>
  );
};

export default LocationContext;
