import { ReactNode, createContext, useEffect, useState } from "react";
import { UiError, Notification } from "../lib/types";
import { invoke } from "@tauri-apps/api";
import { listen } from "@tauri-apps/api/event";
import { useNavigate } from "react-router-dom";
import { handleError } from "../lib/util";
import { info } from "tauri-plugin-log-api";

export interface NotificationContextInterface {
  notificationLoading: boolean;
  notifications: Notification[];
  getNotifications: () => void;
  ackNotification: (id: string) => void;
}

const NotificationContext = createContext<
  NotificationContextInterface | undefined
>(undefined);

export const NotificationProvider = ({ children }: { children: ReactNode }) => {
  const navigate = useNavigate();
  const [notificationLoading, setNotificationLoading] = useState(false);
  const [notifications, setNotifications] = useState([] as Notification[]);

  useEffect(() => {
    const subscribe = async () => {
      info("Subscribed to notification events");
      // subscribe to notification events
      return await listen<Notification>("notification", (event) => {
        let found = notifications.findIndex(
          (notif) => notif.id === event.payload.id
        );
        if (found == -1) {
          let newNotifications = notifications.concat([event.payload]);
          setNotifications(newNotifications);
        }
      });
    };

    const unlisten = subscribe();

    const unsubscribe = async () => {
      info("unsubscribed from notification events");
      (await unlisten)();
    };

    return () => {
      unsubscribe();
    };
  }, [notifications]);

  const getNotifications = () => {
    setNotificationLoading(true);
    const fetchNotifications = async () => {
      try {
        const notifications = (await invoke("notifications")) as Notification[];
        setNotifications(notifications);
      } catch (e) {
        const error = e as UiError;
        handleError(error, navigate);
      }
      setNotificationLoading(false);
    };

    fetchNotifications();
  };

  const ackNotification = (id: string) => {
    const ack = async () => {
      let filtered = notifications.filter((n) => n.id != id);
      setNotifications(filtered);
      try {
        info(`Acking ${id}`);
        await invoke("ack_notification", { id });
      } catch (e) {
        // set notifications back to original
        setNotifications(notifications);
        const error = e as UiError;
        handleError(error, navigate);
      }
    };
    ack();
  };

  return (
    <NotificationContext.Provider
      value={{
        notificationLoading,
        notifications,
        getNotifications,
        ackNotification,
      }}
    >
      {children}
    </NotificationContext.Provider>
  );
};

export default NotificationContext;
