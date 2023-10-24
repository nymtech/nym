import { dir } from "console";
import { readFileSync } from "fs";
import { dirname } from "path";
import { TLogLevelName } from "tslog";

import YAML from "yaml";

class ConfigHandler {
  private static instance: ConfigHandler;

  private validEnvironments = ["sandbox", "prod"];

  public commonConfig: { request_headers: object };

  private currentEnvironment: string;

  public environment: string;

  public environmentConfig: {
    log_level: TLogLevelName;
    time_zone: string;
    api_base_url: string;
    mix_id: number;
    identity_key: string;
    gateway_identity: string;
  };

  private constructor() {
    this.setCommonConfig();
    this.setEnvironmentConfig(process.env.TEST_ENV || "sandbox" || "prod");
  }

  public static getInstance(): ConfigHandler {
    if (!ConfigHandler.instance) {
      ConfigHandler.instance = new ConfigHandler();
    }
    return ConfigHandler.instance;
  }

  private setCommonConfig(): void {
    try {
      const baseWorkingDirectory = __dirname;
      this.commonConfig = YAML.parse(
        readFileSync(baseWorkingDirectory + "/config.yaml", "utf8"),
      ).common;
    } catch (error) {
      throw Error(`Error reading common config: (${error})`);
    }
  }

  private setEnvironmentConfig(environment: string): void {
    this.ensureEnvironmentIsValid(environment);
    try {
      const baseWorkingDirectory = __dirname;
      this.environmentConfig = YAML.parse(
        readFileSync(baseWorkingDirectory + "/config.yaml", "utf8"),
      )[environment];
    } catch (error) {
      console.log("fadsfasdfasdfsdfsa")
      throw Error(`Error reading environment config: (${error})`);
    }
  }

  public getEnvironmentConfig(environment: string): any {
    const baseWorkingDirectory = __dirname;
    return (
      this.environmentConfig ||
      YAML.parse(readFileSync(baseWorkingDirectory + "/config.yaml", "utf8"))[environment]
    );
  }

  private ensureEnvironmentIsValid(environment: string): void {
    if (this.validEnvironments.indexOf(environment) === -1) {
      throw Error(`Config environment is not valid: "${environment}"`);
    }
  }
}

export default ConfigHandler;
