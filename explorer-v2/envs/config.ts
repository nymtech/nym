import { type Environment } from "../src/providers/EnvironmentProvider";

interface EnvConfig {
  envName: Environment;
  basePath: string;
  apiUrl?: string;
}


function getMainnetEnv(): EnvConfig {
  return {
    envName: "mainnet",
    basePath: "/explorer",
    // apiUrl:
    //   process.env.NEXT_PUBLIC_MAINNET_API_URL || "https://nym.com/explorer",
  };
}

function getSandboxEnv(): EnvConfig {
  return {
    envName: "sandbox",
    basePath: "/sandbox-explorer",
    // apiUrl:
    //   process.env.NEXT_PUBLIC_SANDBOX_API_URL ||
    //   "https://nym.com/sandbox-explorer",
  };
}

export const getEnvByName = (name: Environment): EnvConfig => {
  if (name === "sandbox") {
    return getSandboxEnv();
  }
  if (name === "mainnet") {
    return getMainnetEnv();
  }

  // Default to mainnet
  log("🐼 using mainnet env vars");
  return getMainnetEnv();
};


export const getBasePathByEnv = (env: Environment): string => {
  return getEnvByName(env).basePath;
};

// export const getApiUrlByEnv = (env: Environment): string => {
//   return getEnvByName(env).apiUrl;
// };
