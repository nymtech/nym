import React, { useEffect } from "react";
import {
  MdOutlineHome,
  MdOutlineLocationOn,
  MdOutlineManageAccounts,
} from "react-icons/md";
import { useNavigate } from "react-router-dom";

type Props = {
  activeHome?: boolean;
  activeLocation?: boolean;
  activeSettings?: boolean;
  children?: React.ReactNode;
};

function Layout(props: Props) {
  const navigate = useNavigate();

  useEffect(() => {
    if (import.meta.env.PROD) {
      document.addEventListener("contextmenu", (event) =>
        event.preventDefault()
      );
    }
  }, []);

  return (
    <div>
      <div className="h-[calc(100vh-64px)] overflow-y-auto scroll-smooth">
        {props.children}
      </div>
      <div className="btm-nav">
        <button
          className={`text-info ${props.activeLocation ? "active" : ""}`}
          onClick={() => navigate("/locations")}
        >
          <MdOutlineLocationOn size="1.5em" />
        </button>
        <button
          className={`text-info ${props.activeHome ? "active" : ""}`}
          onClick={() => navigate("/")}
        >
          <MdOutlineHome size="1.5em" />
        </button>
        <button
          className={`text-info ${props.activeSettings ? "active" : ""}`}
          onClick={() => navigate("/settings")}
        >
          <MdOutlineManageAccounts size="1.5em" />
        </button>
      </div>
    </div>
  );
}

export default Layout;
