import { BsShieldSlash, BsShieldCheck } from "react-icons/bs";

import { useNavigate } from "react-router-dom";
import Layout from "../components/Layout";
import { ReactNode, useContext, useEffect } from "react";
import LocationContext, {
  LocationContextInterface,
} from "../context/LocationContext";
import LocationSelector from "../components/LocationSelector";
import VpnStatusContext, {
  VpnStatusContextInterface,
} from "../context/VpnStatusContext";
import {
  defaultLocation,
  getLocationFromVpnStatus,
  isVpnInProgress,
} from "../lib/util";
import NotificationList from "../components/NotificationList";
import RecentLocations from "../components/RecentLocations";
import NotificationContext, {
  NotificationContextInterface,
} from "../context/NotificationContext";
import Timer from "../components/Timer";
import { DateTime } from "luxon";

type Props = {};

interface HomeUIState {
  displayShield: ReactNode;
  progressBar: ReactNode;
  vpnStatusMessage: ReactNode;
  locationSelectorEnabled: boolean;
  switchOff: boolean;
  switchEnabled: boolean;
  switchOnChange: () => void;
  toolTip: string;
  waitingToConnect: boolean;
}

function Home({}: Props) {
  const navigate = useNavigate();
  const {
    loadingLocations,
    selected,
    setLocation,
    getLocations,
    locations,
    getRecentLocations,
    recentLocations,
  } = useContext(LocationContext) as LocationContextInterface;

  const { vpnStatus, getVpnStatus, connect, disconnect } = useContext(
    VpnStatusContext
  ) as VpnStatusContextInterface;

  const { notifications, getNotifications, notificationLoading } = useContext(
    NotificationContext
  ) as NotificationContextInterface;

  // Load locations on home page so that data is available for
  // LocationSelector component and Locations page
  useEffect(() => {
    getVpnStatus();
  }, []);

  useEffect(() => {
    getLocations();
  }, []);

  useEffect(() => {
    getNotifications();
  }, []);

  useEffect(() => {
    getRecentLocations();
  }, [vpnStatus]);

  useEffect(() => {
    if (vpnStatus !== undefined) {
      const locationFromStatus = getLocationFromVpnStatus(vpnStatus);
      if (locationFromStatus != undefined) {
        setLocation(locationFromStatus);
      }
    }
  }, [vpnStatus]);

  // recent locations
  let filteredRecentLocations = [];
  for (const recentLocation of recentLocations) {
    for (const location of locations) {
      if (recentLocation.code === location.code) {
        filteredRecentLocations.push(location);
        break;
      }
    }
  }

  // Derive UI state based on vpnStatus

  const defaultHomeUIState: HomeUIState = {
    displayShield: <BsShieldSlash size="4em" />,
    vpnStatusMessage: (
      <div className="badge badge-outline badge-lg badge-neutral">
        VPN is off
      </div>
    ),
    progressBar: (
      <div className="h-8">
        <div className="divider"></div>
      </div>
    ),
    locationSelectorEnabled: true,
    switchEnabled: true && locations.length > 0,
    switchOff: true,
    switchOnChange: () => {
      const location = selected ? selected : defaultLocation(locations);
      if (location) {
        connect(location);
      }
    },
    toolTip: "Turn on VPN",
    waitingToConnect: false,
  };

  const homeUIState = ((): HomeUIState => {
    if (!vpnStatus) {
      return defaultHomeUIState;
    }

    switch (vpnStatus.type) {
      case "Disconnected":
        return defaultHomeUIState;
      case "Accepted":
        return {
          displayShield: <BsShieldSlash size="4em" />,
          vpnStatusMessage: (
            <div className="badge badge-outline badge-lg">{vpnStatus.type}</div>
          ),
          progressBar: (
            <div className="h-8 pt-2">
              <progress
                className="progress progress-info w-64 bg-base-100"
                value="25"
                max="100"
              ></progress>
            </div>
          ),
          locationSelectorEnabled: false,
          switchEnabled: false,
          switchOff: false,
          switchOnChange: () => {},
          toolTip: "Turn off VPN",
          waitingToConnect: true,
        };
      case "Connecting":
        return {
          displayShield: <BsShieldSlash size="4em" />,
          vpnStatusMessage: (
            <div className="badge badge-outline badge-lg">{vpnStatus.type}</div>
          ),
          progressBar: (
            <div className="h-8 pt-2">
              <progress
                className="progress progress-info w-64 bg-base-100"
                value="95"
                max="100"
              ></progress>
            </div>
          ),
          locationSelectorEnabled: false,
          switchEnabled: false,
          switchOff: false,
          switchOnChange: () => {},
          toolTip: "Turn off VPN",
          waitingToConnect: true,
        };
      case "ServerReady":
        return {
          displayShield: <BsShieldSlash size="4em" />,
          vpnStatusMessage: (
            <div className="badge badge-outline badge-lg">{vpnStatus.type}</div>
          ),
          progressBar: (
            <div className="h-8 pt-2">
              <progress
                className="progress progress-info w-64 bg-base-100"
                value="80"
                max="100"
              ></progress>
            </div>
          ),
          locationSelectorEnabled: false,
          switchEnabled: false,
          switchOff: false,
          switchOnChange: () => {},
          toolTip: "Turn off VPN",
          waitingToConnect: true,
        };
      case "ServerCreated":
        return {
          displayShield: <BsShieldSlash size="4em" />,
          vpnStatusMessage: (
            <div className="badge badge-outline badge-lg">{vpnStatus.type}</div>
          ),
          progressBar: (
            <div className="h-8 pt-2">
              <progress
                className="progress progress-info w-64 bg-base-100"
                value="50"
                max="100"
              ></progress>
            </div>
          ),
          locationSelectorEnabled: false,
          switchEnabled: false,
          switchOff: false,
          switchOnChange: () => {},
          toolTip: "Turn off VPN",
          waitingToConnect: true,
        };
      case "ServerRunning":
        return {
          displayShield: <BsShieldSlash size="4em" />,
          vpnStatusMessage: (
            <div className="badge badge-outline badge-lg">{vpnStatus.type}</div>
          ),
          progressBar: (
            <div className="h-8 pt-2">
              <progress
                className="progress progress-info w-64 bg-base-100"
                value="75"
                max="100"
              ></progress>
            </div>
          ),
          locationSelectorEnabled: false,
          switchEnabled: false,
          switchOff: false,
          switchOnChange: () => {},
          toolTip: "Turn off VPN",
          waitingToConnect: true,
        };
      case "Connected":
        return {
          displayShield: <BsShieldCheck size="4em" className="text-success" />,
          vpnStatusMessage: (
            <div className="badge badge-outline badge-lg">VPN is on</div>
          ),
          progressBar: (
            <div className="h-8">
              <div className="divider">
                <Timer connectedTime={DateTime.fromISO(vpnStatus.payload[1])} />
              </div>
            </div>
          ),
          locationSelectorEnabled: false,
          switchEnabled: true,
          switchOff: false,
          switchOnChange: () => {
            const location = selected ? selected : locations[0];
            disconnect(location);
          },
          toolTip: "Turn off VPN",
          waitingToConnect: false,
        };
      case "Disconnecting":
        return {
          displayShield: <BsShieldSlash size="4em" />,
          vpnStatusMessage: (
            <div className="badge badge-outline badge-lg">{vpnStatus.type}</div>
          ),
          progressBar: (
            <div className="h-8 pt-2">
              <div className="divider"></div>
            </div>
          ),
          locationSelectorEnabled: false,
          switchEnabled: false,
          switchOff: true,
          switchOnChange: () => {},
          toolTip: "Turn on VPN",
          waitingToConnect: false,
        };

      default:
        break;
    }

    return defaultHomeUIState;
  })();

  const recentLocationsDisabled = isVpnInProgress(vpnStatus);

  return (
    <Layout activeHome={true}>
      <div>
        <div className="mx-3 mt-3">
          <div className="card bg-base-300">
            <div className="card-body items-center text-center py-6">
              <figure>{homeUIState.displayShield}</figure>
              <h2 className="card-title mt-4">
                <div className="flex flex-col">
                  <div className="px-2">{homeUIState.vpnStatusMessage}</div>
                  <div className="w-64 h-8 mb-2">{homeUIState.progressBar}</div>
                </div>
              </h2>
              <LocationSelector
                enabled={homeUIState.locationSelectorEnabled}
                waitingToConnect={homeUIState.waitingToConnect}
              />
              <div className="card-actions justify-center mt-2">
                <div
                  className="tooltip tooltip-bottom"
                  data-tip={homeUIState.toolTip}
                >
                  <input
                    type="checkbox"
                    className="toggle toggle-accent"
                    checked={!homeUIState.switchOff}
                    onChange={homeUIState.switchOnChange}
                    disabled={!homeUIState.switchEnabled}
                  />
                </div>
              </div>
            </div>
          </div>
        </div>
        <div className="mx-3 mt-3">
          <div
            className={
              notifications.length > 0 && !notificationLoading
                ? "block"
                : "hidden"
            }
          >
            {<NotificationList notifications={notifications} />}
          </div>
          <div
            className={
              notifications.length == 0 && filteredRecentLocations.length > 0
                ? "block"
                : "hidden"
            }
          >
            <RecentLocations
              locations={filteredRecentLocations}
              disabled={recentLocationsDisabled}
            />
          </div>
        </div>
      </div>
    </Layout>
  );
}

export default Home;
