"use client";
import React, { createContext, useContext, useEffect, useState } from "react";

type Environment = "mainnet" | "sandbox";

interface EnvironmentContextType {
  environment: Environment;
  setEnvironment: (env: Environment) => void;
}

const EnvironmentContext = createContext<EnvironmentContextType | undefined>(
  undefined
);

const ENVIRONMENT_STORAGE_KEY = "environment";

export const EnvironmentProvider: React.FC<{ children: React.ReactNode }> = ({
  children,
}) => {
  const [environment, setEnvironmentState] = useState<Environment>(() => {
    // Try to get the environment from localStorage on initial load
    const storedEnv = localStorage.getItem(ENVIRONMENT_STORAGE_KEY);
    return (storedEnv as Environment) || "mainnet";
  });

  const setEnvironment = (env: Environment) => {
    setEnvironmentState(env);
    localStorage.setItem(ENVIRONMENT_STORAGE_KEY, env);
  };

  // Update localStorage when environment changes
  useEffect(() => {
    localStorage.setItem(ENVIRONMENT_STORAGE_KEY, environment);
  }, [environment]);

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
