import { NavigateFunction } from "react-router-dom";
import { VpnStatus, Location, UiError, Code } from "./types";
import toast from "react-hot-toast";
import { error as logError, info } from "tauri-plugin-log-api";
import { type } from '@tauri-apps/api/os';
import { invoke } from "@tauri-apps/api";
import { isPermissionGranted, requestPermission } from "@tauri-apps/api/notification";

export function getLocationFromVpnStatus(status: VpnStatus): Location | undefined {
    switch (status.type) {
        case "Accepted":
        case "Connecting":
        case "Disconnecting":
        case "ServerRunning":
        case "ServerReady":
            return status.payload
        case "Connected":
            return status.payload[0]
        default:
            return undefined;
    }
}

export const isVpnInProgress = (vpnStatus: VpnStatus | undefined) => {
    if (vpnStatus === undefined) {
        return false;
    }

    switch (vpnStatus.type) {
        case "Accepted":
        case "Connecting":
        case "Connected":
        case "Disconnecting":
        case "ServerRunning":
        case "ServerReady":
        case "ServerCreated":
            return true;

        default:
            break;
    }

    return false;
};

export const isOffline = (error: UiError): boolean => {
    if (error.type === "DaemonIsOffline") {
        return true;
    }
    return false;
}

export const isUnauthenticated = (error: UiError): boolean => {
    if (error.type === "Grpc" && error.code === Code.Unauthenticated) {
        return true;
    }
    return false;
}


export const handleError = (error: UiError, navigate: NavigateFunction, toastWhenUnauthenticated: boolean = false) => {
    switch (error.type) {
        case "DaemonIsOffline":
            navigate("/daemon-offline");
            break;
        case "Grpc":
            logError(error.message);
            if (error.code === Code.Unauthenticated) {
                if (toastWhenUnauthenticated) {
                    toast.error(error.message);
                }
                navigate("/sign-in");
            } else {
                logError(`${error.code} ${error.message}`);
                toast.error(error.message);
            }
    }
}

export const send_desktop_notification = async (message: string): Promise<boolean> => {
    const osType = await type();
    switch (osType) {
        case "Linux":
            await invoke("send_desktop_notification", {
                title: "nymvpn",
                body: message,
            })
            return true;
        case "Darwin":
            let permissionGranted = await isPermissionGranted();
            info(`permissionGranted: ${permissionGranted}`);
            if (!permissionGranted) {
                const permission = await requestPermission();
                info(`permission: ${permission}`);
                permissionGranted = permission === "granted";
            }
            if (permissionGranted) {
                await invoke("send_desktop_notification", {
                    title: "nymvpn",
                    body: message,
                });
                return true;
            }
            break;
        case "Windows_NT":
            break;
        default:
            break;
    }
    return false
}

export const defaultLocation = (locations: Location[]): undefined | Location => {
    return locations.find((value) => {
        return value.city.includes("Ashburn") || value.city.includes("Hillsboro");
    })
}
