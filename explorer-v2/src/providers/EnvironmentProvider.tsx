"use client";
import React, { createContext, useContext } from "react";
import dynamic from "next/dynamic";

type Environment = "mainnet" | "sandbox";

interface EnvironmentContextType {
  environment: Environment;
  setEnvironment: (env: Environment) => void;
}

const EnvironmentContext = createContext<EnvironmentContextType | undefined>(
  undefined
);

const ENVIRONMENT_STORAGE_KEY = "environment";

const ClientStorage = dynamic(
  () =>
    import("@uidotdev/usehooks").then((mod) => {
      const { useLocalStorage } = mod;
      return function ClientStorageComponent({
        children,
      }: {
        children: React.ReactNode;
      }) {
        const [stored, setStored] = useLocalStorage<{ env: Environment }>(
          ENVIRONMENT_STORAGE_KEY,
          { env: "mainnet" }
        );

        const setEnvironment = (env: Environment) => {
          setStored({ env });
        };

        return (
          <EnvironmentContext.Provider
            value={{ environment: stored.env, setEnvironment }}
          >
            {children}
          </EnvironmentContext.Provider>
        );
      };
    }),
  { ssr: false }
);

export const EnvironmentProvider: React.FC<{ children: React.ReactNode }> = ({
  children,
}) => {
  return <ClientStorage>{children}</ClientStorage>;
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
