
export interface Location {
    code: string;
    country: string;
    country_code: string;
    city: string;
    city_code: string;
    state?: string;
    state_code?: string;
}

export type VpnStatus =
    | { type: "Accepted"; payload: Location }
    | { type: "ServerCreated"; payload: Location }
    | { type: "ServerRunning"; payload: Location }
    | { type: "ServerReady"; payload: Location }
    | { type: "Connecting"; payload: Location }
    | { type: "Connected"; payload: [Location, string] }
    | { type: "Disconnecting"; payload: Location }
    | { type: "Disconnected" };

export type NotificationType = "ServerFailed" | "ClientFailed";

export interface Notification {
    id: string;
    message: string;
    notification_type: NotificationType;
    timestamp: string;
}

export type UiError =
    | { type: "DaemonIsOffline" }
    | { type: "Grpc"; code: number; message: string };

export enum Code {
    Unauthenticated = 16
}
