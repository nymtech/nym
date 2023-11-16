import React from "react";
import useAuthStatus from "../hooks/useAuthStatus";
import { Navigate, Outlet } from "react-router-dom";
import Spinner from "./Spinner";
import Hero from "./Hero";

function ProtectedRoute() {
  const [checkingAuthStatus, daemonOffline, signedIn] = useAuthStatus();

  if (daemonOffline) {
    return <Navigate to="/daemon-offline" />;
  }

  if (signedIn) {
    return <Outlet />;
  }

  if (checkingAuthStatus) {
    return (
      <Hero>
        <Spinner className="w-24 h-24" />
      </Hero>
    );
  } else {
    return <Navigate to="/sign-in" />;
  }
}

export default ProtectedRoute;
