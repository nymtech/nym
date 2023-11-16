import { IoMdRefresh } from "react-icons/io";
import { useNavigate } from "react-router-dom";
import Hero from "../components/Hero";
import { useEffect } from "react";

function DaemonOffline() {
  const navigate = useNavigate();
  const onRetry = () => {
    navigate("/");
  };

  useEffect(() => {
    if (import.meta.env.PROD) {
      document.addEventListener("contextmenu", (event) =>
        event.preventDefault()
      );
    }
  }, []);

  return (
    <Hero>
      <p className="font-bold text-error">Daemon is offline</p>
      <div className="form-control mt-6">
        <button onClick={onRetry} className="btn btn-error btn-wide">
          Retry <IoMdRefresh className="mx-2 text-xl" />
        </button>
      </div>
    </Hero>
  );
}

export default DaemonOffline;
