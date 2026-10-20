// `<Output name="command-outputs/nym-node-help" />`: a file that the prebuild or
// the autodoc job writes under components/outputs/ (a `--help` text, build
// info, a network number, a table). A snippet imports nothing, so it names the
// file by its path below components/outputs/, with no extension. The text
// outputs leave these files out, as they left out the `.md` imports before
// (`inlineMdxPartials` inlines `.mdx` imports only).
// Each name a snippet uses must be in OUTPUTS; a test checks it.

import type { ComponentType } from "react";
import ApiScrapingOutputsNyxOutputsCirculatingSupply from "./outputs/api-scraping-outputs/nyx-outputs/circulating-supply.md";
import ApiScrapingOutputsNyxOutputsEpochRewardBudget from "./outputs/api-scraping-outputs/nyx-outputs/epoch-reward-budget.md";
import ApiScrapingOutputsNyxOutputsStakeSaturation from "./outputs/api-scraping-outputs/nyx-outputs/stake-saturation.md";
import ApiScrapingOutputsNyxOutputsStakingScaleFactor from "./outputs/api-scraping-outputs/nyx-outputs/staking-scale-factor.md";
import ApiScrapingOutputsNyxOutputsStakingTarget from "./outputs/api-scraping-outputs/nyx-outputs/staking-target.md";
import ApiScrapingOutputsNyxOutputsStakingSupply from "./outputs/api-scraping-outputs/nyx-outputs/staking_supply.md";
import ApiScrapingOutputsNyxOutputsTokenTable from "./outputs/api-scraping-outputs/nyx-outputs/token-table.md";
import CommandOutputsNymApiBuildInfo from "./outputs/command-outputs/nym-api-build-info.md";
import CommandOutputsNymApiHelp from "./outputs/command-outputs/nym-api-help.md";
import CommandOutputsNymNodeCliInstallHelp from "./outputs/command-outputs/nym-node-cli-install-help.md";
import CommandOutputsNymvisorBuildInfo from "./outputs/command-outputs/nymvisor-build-info.md";
import CommandOutputsNymvisorHelp from "./outputs/command-outputs/nymvisor-help.md";
import Csv2mdOutputsIspSheet from "./outputs/csv2md-outputs/isp-sheet.md";
import NyxOutputsNyxPercentStake from "./outputs/nyx-outputs/nyx-percent-stake.md";
import NyxOutputsNyxTotalStake from "./outputs/nyx-outputs/nyx-total-stake.md";

const OUTPUTS: Record<string, ComponentType> = {
  "api-scraping-outputs/nyx-outputs/circulating-supply": ApiScrapingOutputsNyxOutputsCirculatingSupply,
  "api-scraping-outputs/nyx-outputs/epoch-reward-budget": ApiScrapingOutputsNyxOutputsEpochRewardBudget,
  "api-scraping-outputs/nyx-outputs/stake-saturation": ApiScrapingOutputsNyxOutputsStakeSaturation,
  "api-scraping-outputs/nyx-outputs/staking-scale-factor": ApiScrapingOutputsNyxOutputsStakingScaleFactor,
  "api-scraping-outputs/nyx-outputs/staking-target": ApiScrapingOutputsNyxOutputsStakingTarget,
  "api-scraping-outputs/nyx-outputs/staking_supply": ApiScrapingOutputsNyxOutputsStakingSupply,
  "api-scraping-outputs/nyx-outputs/token-table": ApiScrapingOutputsNyxOutputsTokenTable,
  "command-outputs/nym-api-build-info": CommandOutputsNymApiBuildInfo,
  "command-outputs/nym-api-help": CommandOutputsNymApiHelp,
  "command-outputs/nym-node-cli-install-help": CommandOutputsNymNodeCliInstallHelp,
  "command-outputs/nymvisor-build-info": CommandOutputsNymvisorBuildInfo,
  "command-outputs/nymvisor-help": CommandOutputsNymvisorHelp,
  "csv2md-outputs/isp-sheet": Csv2mdOutputsIspSheet,
  "nyx-outputs/nyx-percent-stake": NyxOutputsNyxPercentStake,
  "nyx-outputs/nyx-total-stake": NyxOutputsNyxTotalStake,
};

export function Output({ name }: { name: string }) {
  const Component = Object.hasOwn(OUTPUTS, name) ? OUTPUTS[name] : null;
  if (!Component) throw new Error(`<Output name="${name}">: not one of the outputs in components/output.tsx`);
  return <Component />;
}
