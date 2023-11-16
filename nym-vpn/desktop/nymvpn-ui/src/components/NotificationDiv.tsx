import React, { useContext } from "react";
import { Notification } from "../lib/types";
import { DateTime } from "luxon";
import NotificationContext, {
  NotificationContextInterface,
} from "../context/NotificationContext";

type Props = {
  notification: Notification;
};

const NotificationDiv = ({ notification }: Props) => {
  const { ackNotification } = useContext(
    NotificationContext
  ) as NotificationContextInterface;

  return (
    <div className="card shadow-xl bg-base-300 max-h-40">
      <div className="card-body px-6 py-5">
        <h2 className="card-title">
          <p className="text-info">Oh No</p>

          <div>
            <button
              className="btn btn-square btn-ghost btn-sm"
              onClick={() => ackNotification(notification.id)}
            >
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
                  d="M6 18L18 6M6 6l12 12"
                />
              </svg>
            </button>
          </div>
        </h2>

        <p className="text-md max-h-12 overflow-y-auto">
          {notification.message}
        </p>

        <p className="text-xs text-accent font-bold">
          {DateTime.fromISO(notification.timestamp).toLocaleString(
            DateTime.DATETIME_MED
          )}
        </p>
      </div>
    </div>
  );
};

export default NotificationDiv;
