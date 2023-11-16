import React, { useContext, useEffect, useState } from "react";
import Layout from "../components/Layout";
import { info } from "tauri-plugin-log-api";
import { open } from "@tauri-apps/api/shell";
import { useNavigate } from "react-router";
import Spinner from "../components/Spinner";
import { invoke } from "@tauri-apps/api";
import VpnStatusContext, {
  VpnStatusContextInterface,
} from "../context/VpnStatusContext";
import { handleError, isVpnInProgress } from "../lib/util";
import { UiError } from "../lib/types";
import { toast } from "react-hot-toast";
import Navbar from "../components/Navbar";
import { Link } from "react-router-dom";
import { MdOpenInNew } from "react-icons/md";

type Props = {};

function Settings({}: Props) {
  const [signingOut, setSigningOut] = useState(false);
  const [appVersion, setAppVersion] = useState("");
  const [updateAvailable, setUpdateAvailable] = useState(false);

  const { vpnStatus } = useContext(
    VpnStatusContext
  ) as VpnStatusContextInterface;

  const navigate = useNavigate();

  const inProgress = isVpnInProgress(vpnStatus);

  const onClick = () => {
    if (inProgress) {
      toast.error!("Cannot sign out when VPN session is in progress");
      return;
    }

    setSigningOut(true);
    const signOut = async () => {
      try {
        await invoke("sign_out");
        navigate("/sign-in");
      } catch (e) {
        const error = e as UiError;
        handleError(error, navigate);
      }
      setSigningOut(false);
    };
    signOut();
  };

  useEffect(() => {
    const fetchVersion = async () => {
      try {
        const currentVersion = await invoke<string>("current_app_version");
        setAppVersion(currentVersion);
      } catch (e) {}
    };

    fetchVersion();
  }, []);

  useEffect(() => {
    const isUpdateAvailable = async () => {
      try {
        const isAvailable = await invoke<boolean>("update_available");
        setUpdateAvailable(isAvailable);
      } catch (e) {
        const error = e as UiError;
        handleError(error, navigate);
      }
    };
    isUpdateAvailable();
  }, []);

  const showOSSLicenses = () => {
    const showLicense = async () => {
      await invoke("open_license");
    };
    showLicense();
  };

  const showLogFile = () => {
    const showLicense = async () => {
      await invoke("open_log_file");
    };
    showLicense();
  };

  return (
    <Layout activeSettings={true}>
      <div className="flex flex-col h-full">
        <Navbar header="Account and Settings" />
        <div className="mx-2">
          <ul className="menu bg-base-100 p-1 gap-1 rounded-box">
            <li>
              <a
                href={`${import.meta.env.NYMVPN_URL}/dashboard`}
                target="_blank"
                className="flex flex-row justify-between"
              >
                <span>Dashboard</span>

                <MdOpenInNew size="1.5em" />
              </a>
            </li>
            <li onClick={showLogFile}>
              <div className="flex flex-row justify-between">
                <span>View Logs</span>

                <MdOpenInNew size="1.5em" />
              </div>
            </li>
            <li onClick={showOSSLicenses}>
              <div className="flex flex-row justify-between">
                <span>View Open Source Licenses</span>

                <MdOpenInNew size="1.5em" />
              </div>
            </li>

            <li className={`${inProgress || signingOut ? "disabled" : ""}`}>
              <div onClick={onClick}>
                <div>{signingOut ? <Spinner /> : <span>Sign Out</span>}</div>
              </div>
            </li>
          </ul>
        </div>
        <div className="flex-1 mb-5">
          <div className="flex flex-col gap-2 h-full justify-end">
            <a
              className={`self-center btn btn-ghost btn-wide gap-2 ${
                updateAvailable ? "" : "hidden"
              }`}
              href={`${import.meta.env.NYMVPN_URL}/download`}
              target="_blank"
            >
              <p>Update available</p>
              <MdOpenInNew size="1.5em" />
            </a>
            <div
              className={`self-center badge badge-lg text-info ${
                appVersion.length > 0 ? "" : "hidden"
              }`}
            >
              Version: {appVersion}
            </div>
          </div>
        </div>
      </div>
    </Layout>
  );
}

export default Settings;
