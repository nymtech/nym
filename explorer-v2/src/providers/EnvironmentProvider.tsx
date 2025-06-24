"use client";
import React, { createContext, useContext, useState, useEffect } from "react";
import { usePathname } from "next/navigation";

export type Environment = "mainnet" | "sandbox";

interface EnvironmentContextType {
  environment: Environment;
  setEnvironment: (env: Environment) => void;
}

const EnvironmentContext = createContext<EnvironmentContextType | undefined>(
  undefined
);

export const EnvironmentProvider: React.FC<{ children: React.ReactNode }> = ({
  children,
}) => {
  const [environment, setEnvironment] = useState<Environment>("mainnet");
  const pathname = usePathname();

  // Initialize environment from URL path
  useEffect(() => {
    if (pathname.startsWith("/sandbox-explorer")) {
      setEnvironment("sandbox");
    } else if (pathname.startsWith("/explorer")) {
      setEnvironment("mainnet");
    } else {
      // Default to mainnet for other paths
      setEnvironment("mainnet");
    }
  }, [pathname]);

  return (
    <EnvironmentContext.Provider value={{ environment, setEnvironment }}>
      {children}
    </EnvironmentContext.Provider>
  );
};

export const useEnvironment = () => {
  const context = useContext(EnvironmentContext);
  if (context === undefined) {
    throw new Error(
      "useEnvironment must be used within an EnvironmentProvider"
    );
  }
  return context;
};
