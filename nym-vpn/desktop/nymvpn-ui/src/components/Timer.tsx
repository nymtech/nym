import React, { useContext, useEffect, useState } from "react";
import { DateTime } from "luxon";

type Props = {
  connectedTime: DateTime;
};

const Timer = ({ connectedTime }: Props) => {
  const [diff, setDiff] = useState(
    DateTime.now().diff(connectedTime, ["days", "hours", "minutes", "seconds"])
  );

  useEffect(() => {
    const interval = setInterval(() => {
      let newDiff = DateTime.now().diff(connectedTime, [
        "days",
        "hours",
        "minutes",
        "seconds",
      ]);
      setDiff(newDiff);
    }, 1000);
    return () => clearInterval(interval);
  }, []);

  const days = diff.days.toLocaleString("en-US", { minimumIntegerDigits: 2 });
  const hours = diff.hours.toLocaleString("en-US", { minimumIntegerDigits: 2 });
  const minutes = diff.minutes.toLocaleString("en-US", {
    minimumIntegerDigits: 2,
  });
  const seconds = diff.seconds.toLocaleString("en-US", {
    minimumIntegerDigits: 2,
    maximumFractionDigits: 0,
  });

  if (diff.days > 0) {
    return (
      <div className="badge badge-info">
        <div className="w-24">
          <span>{days}</span>
          <span>:</span>
          <span>{hours}</span>
          <span>:</span>
          <span>{minutes}</span>
          <span>:</span>
          <span>{seconds}</span>
        </div>
      </div>
    );
  }

  return (
    <div className="badge badge-info">
      <div className="w-16">
        <span>{hours}</span>
        <span>:</span>
        <span>{minutes}</span>
        <span>:</span>
        <span>{seconds}</span>
      </div>
    </div>
  );
};

export default Timer;
