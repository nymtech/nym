import { type Environment } from "../src/providers/EnvironmentProvider";

interface EnvConfig {
  envName: Environment;
  basePath: string;
  apiUrl?: string;
}

function log(message?: any, ...optionalParams: any[]) {
  if (
    process.env.NODE_ENV === "development" ||
    process.env.DEBUG_CONFIG_LOGS === "true"
  ) {
    console.log(message, ...optionalParams);
  }
}

// export function getCurrentEnv(): Environment {
//   // Check for VERCEL_ENV from .env file
//   if (process.env.VERCEL_ENV === "sandbox") {
//     return "sandbox";
//   }
//   if (process.env.VERCEL_ENV === "production") {
//     return "mainnet";
//   }

//   // Check for environment-specific deployment branches
//   if (process.env.VERCEL_GIT_COMMIT_REF === "deploy/sandbox") {
//     return "sandbox";
//   }
//   if (process.env.VERCEL_GIT_COMMIT_REF === "deploy/mainnet") {
//     return "mainnet";
//   }

//   // Check for NODE_ENV
//   if (process.env.NODE_ENV === "production") {
//     return "mainnet";
//   }

//   // Default to mainnet for development
//   return "mainnet";
// }

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

// export const getEnv = (): EnvConfig => {
//   const currentEnv = getCurrentEnv();
//   log(`currentEnv = "${currentEnv}"`);
//   return getEnvByName(currentEnv);
// };

export const getBasePathByEnv = (env: Environment): string => {
  return getEnvByName(env).basePath;
};

// export const getApiUrlByEnv = (env: Environment): string => {
//   return getEnvByName(env).apiUrl;
// };
