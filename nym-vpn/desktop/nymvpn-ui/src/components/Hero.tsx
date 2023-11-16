import React from "react";
import { ReactComponent as Logo } from "../assets/nymvpn.svg";

type Props = {
  children?: React.ReactNode;
};

function Hero({ children }: Props) {
  return (
    <div className="hero min-h-screen bg-base-200 select-none">
      <div className="hero-content flex-col lg:flex-row-reverse">
        <Logo className="w-12 h-12" />
        <div className="text-center lg:text-left">
          <h2 className="text-5xl">nymvpn</h2>
          <p className="py-6 font-bold">A Modern Serverless VPN</p>
        </div>
        {children}
      </div>
    </div>
  );
}

export default Hero;
