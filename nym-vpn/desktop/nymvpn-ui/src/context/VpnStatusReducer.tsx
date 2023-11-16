import { Location, VpnStatus } from "../lib/types";

export interface VpnStatusState {
  loading: boolean;
  vpnStatus: VpnStatus;
}

export enum VpnStatusActionKind {
  Connect = "CONNECT",
  ConnectResponse = "CONNECT_RESPONSE",
  Disconnect = "DISCONNECT",
  DisconnectResponse = "DISCONNECT_RESPONSE",
  GetVpnStatus = "GET_VPN_STATUS",
  GetVpnStatusResponse = "GET_VPN_STATUS_RESPONSE",
  VpnStatusEvent = "VPN_STATUS_EVENT",
}

export interface VpnStatusAction {
  type: VpnStatusActionKind;
  payload: Location | number | string | undefined | VpnStatus;
}

const VpnStatusReducer = (
  state: VpnStatusState,
  action: VpnStatusAction
): VpnStatusState => {
  const { type, payload } = action;

  switch (type) {
    case VpnStatusActionKind.Connect:
      return {
        ...state,
        loading: true,
        vpnStatus: { type: "Accepted", payload: payload as Location },
      };
    case VpnStatusActionKind.ConnectResponse:
      return {
        ...state,
        loading: false,
        vpnStatus: payload as VpnStatus,
      };
    case VpnStatusActionKind.Disconnect:
      return {
        ...state,
        loading: true,
        vpnStatus: { type: "Disconnecting", payload: payload as Location },
      };
    case VpnStatusActionKind.DisconnectResponse:
      return {
        ...state,
        loading: false,
        vpnStatus: payload as VpnStatus,
      };
    case VpnStatusActionKind.GetVpnStatus:
      return {
        ...state,
        loading: true,
        vpnStatus: payload as VpnStatus,
      };
    case VpnStatusActionKind.GetVpnStatusResponse:
      return {
        ...state,
        loading: false,
        vpnStatus: payload as VpnStatus,
      };

    case VpnStatusActionKind.VpnStatusEvent:
      return {
        ...state,
        vpnStatus: payload as VpnStatus,
      };

    default:
      break;
  }

  return state;
};

export default VpnStatusReducer;
