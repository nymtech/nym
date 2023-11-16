import { HashRouter, Route, Routes } from "react-router-dom";
import SignIn from "./pages/SignIn";
import DaemonOffline from "./pages/DaemonOffline";
import { Toaster } from "react-hot-toast";
import Home from "./pages/Home";
import ProtectedRoute from "./components/ProtectedRoute";
import Locations from "./pages/Locations";
import { LocationProvider } from "./context/LocationContext";
import { VpnStatusProvider } from "./context/VpnStatusContext";
import Settings from "./pages/Settings";
import { NotificationProvider } from "./context/NotificationContext";

function App() {
  return (
    <HashRouter>
      <LocationProvider>
        <VpnStatusProvider>
          <NotificationProvider>
            <Routes>
              <Route element={<ProtectedRoute />}>
                <Route path="/" element={<Home />} />
                <Route path="/locations" element={<Locations />} />
                <Route path="/settings" element={<Settings />} />
              </Route>
              <Route path="/daemon-offline" element={<DaemonOffline />} />
              <Route path="/sign-in" element={<SignIn />} />
            </Routes>
          </NotificationProvider>
        </VpnStatusProvider>
      </LocationProvider>
      <Toaster />
    </HashRouter>
  );
}

export default App;
