import { readFileSync } from "fs";
import { TLogLevelName } from "tslog";
import YAML from "yaml";
import * as dotenv from 'dotenv'; 
import path from "path";

class ConfigHandler {
  private static instance: ConfigHandler;

  private validEnvironments = ["sandbox", "prod"];
  private baseWorkingDirectory: string;

  public commonConfig: { request_headers: object };
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
    this.baseWorkingDirectory = __dirname;
    const environment = process.env.TEST_ENV || "sandbox" || "prod";
    this.loadEnvironment(environment);
  }
  
  private loadEnvironment(environment: string): void {
    this.loadEnvironmentVariables(environment);
    this.setCommonConfig();
    this.setEnvironmentConfig(environment);
  }

  private loadEnvironmentVariables(environment: string): void {
    const envFileName = `${environment}.env`;
    const envFilePath = path.resolve(this.baseWorkingDirectory, `../${envFileName}`);
    dotenv.config({ path: envFilePath });
  }

  public static getInstance(): ConfigHandler {
    if (!ConfigHandler.instance) {
      ConfigHandler.instance = new ConfigHandler();
    }
    return ConfigHandler.instance;
  }

  private setCommonConfig(): void {
    try {
      this.commonConfig = this.readConfigFile().common;
    } catch (error) {
      throw Error(`Error reading common config: (${error})`);
    }
  }

  private setEnvironmentConfig(environment: string): void {
    this.ensureEnvironmentIsValid(environment);
    try {
      this.environmentConfig = this.readConfigFile()[environment];
    } catch (error) {
      throw Error(`Error reading environment config: (${error})`);
    }
  }

  private readConfigFile(): any {
    return YAML.parse(
      readFileSync(path.join(this.baseWorkingDirectory, "/config.yaml"), "utf8")
    );
  }

  private ensureEnvironmentIsValid(environment: string): void {
    if (this.validEnvironments.indexOf(environment) === -1) {
      throw Error(`Config environment is not valid: "${environment}"`);
    }
  }
}

export default ConfigHandler;
