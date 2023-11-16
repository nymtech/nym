import React, { useEffect, useState } from "react";
import { IoMdLogIn } from "react-icons/io";
import { GiPineTree } from "react-icons/gi";
import { invoke } from "@tauri-apps/api";
import { useNavigate } from "react-router-dom";
import toast from "react-hot-toast";
import Spinner from "../components/Spinner";
import { info } from "tauri-plugin-log-api";
import { handleError } from "../lib/util";
import { UiError } from "../lib/types";
import useAuthStatus from "../hooks/useAuthStatus";
import { ReactComponent as Logo } from "../assets/nymvpn.svg";

const SignIn = () => {
  const navigate = useNavigate();
  const [checkingAuthStatus, daemonOffline, signedIn] = useAuthStatus();

  const [checking, setChecking] = useState(false);
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");

  useEffect(() => {
    if (import.meta.env.PROD) {
      document.addEventListener("contextmenu", (event) =>
        event.preventDefault()
      );
    }
  }, []);

  useEffect(() => {
    if (signedIn) {
      navigate("/");
    }
  }, [signedIn]);

  const onSubmit = (e: React.SyntheticEvent) => {
    e.preventDefault();
    const signIn = async () => {
      setChecking(true);
      try {
        info("signing in ...");
        const success = await invoke("sign_in", {
          email,
          password,
        });
        navigate("/");
      } catch (e) {
        const error = e as UiError;
        handleError(error, navigate, true);
      }
      setChecking(false);
    };

    signIn();
  };

  return (
    <div className="min-h-screen bg-base-200 select-none">
      <div className="hero-content flex-col lg:flex-row-reverse">
        <Logo className="w-12 h-12" />
        <div className="text-center lg:text-left">
          <h2 className="text-5xl">nymvpn</h2>
          <p className="py-4 font-bold">A Modern Serverless VPN</p>
        </div>
        <div className="card flex-shrink-0 w-full max-w-sm shadow-2xl bg-base-100">
          <div className="card-body">
            <form onSubmit={onSubmit} >
              <div className="form-control">
                <label htmlFor="email" className="label">
                  <span className="label-text">Email</span>
                </label>

                <input
                  name="email"
                  type="email"
                  autoCorrect="off"
                  autoCapitalize="none"
                  autoComplete="email"
                  onChange={(e) => setEmail(e.target.value)}
                  className="input input-bordered focus:ring-1 focus:outline-none hover:ring-1"
                  disabled={checking}
                  required
                  autoFocus={true}
                />
              </div>
              <div className="form-control">
                <label htmlFor="password" className="label">
                  <span className="label-text">Password</span>
                </label>
                <input
                  name="password"
                  type="password"
                  id="current-password"
                  autoComplete="current-password"
                  onChange={(e) => setPassword(e.target.value)}
                  className="input input-bordered focus:ring-1 focus:outline-none hover:ring-1"
                  disabled={checking}
                  required
                />
                <label className="label">
                  <a
                    href={import.meta.env.NYMVPN_URL}
                    target="_blank"
                    className="label-text-alt link link-hover"
                  >
                    Need an account?
                  </a>
                </label>
              </div>
              <div className="form-control mt-6">
                <button className="btn btn-primary" disabled={checking} >
                  {checking ? <Spinner className="w-12 h-12" /> : <>Sign In</>}
                </button>
              </div>
            </form>
          </div>
        </div>
      </div>
    </div>
  );
};

export default SignIn;
