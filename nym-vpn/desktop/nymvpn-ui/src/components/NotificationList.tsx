import React from "react";
import { Notification } from "../lib/types";
import NotificationDiv from "./NotificationDiv";

type Props = {
  notifications: Notification[];
};

const NotificationList = ({ notifications }: Props) => {
  return (
    <div className="stack max-h-40 items-center w-full overflow-scroll-y">
      {notifications.map((notification) => {
        return (
          <NotificationDiv key={notification.id} notification={notification} />
        );
      })}
    </div>
  );
};

export default NotificationList;
