import { Callout } from "nextra/components";
import { NYM_SDK_VERSION, SMOLMIX_VERSION } from "./versions";

const VERSIONS: Record<string, string> = {
  "nym-sdk": NYM_SDK_VERSION,
  smolmix: SMOLMIX_VERSION,
};

const EXAMPLES_URLS: Record<string, string> = {
  "nym-sdk":
    "https://github.com/nymtech/nym/tree/develop/sdk/rust/nym-sdk/examples",
  smolmix:
    "https://github.com/nymtech/nym/tree/develop/smolmix/core/examples",
};

interface CodeVerifiedProps {
  /** Crate name to display. Defaults to "nym-sdk". */
  crate?: keyof typeof VERSIONS;
}

export const CodeVerified = ({ crate: crateName = "nym-sdk" }: CodeVerifiedProps) => (
  <Callout type="info">
    Code verified against{" "}
    <code>{crateName}</code> v{VERSIONS[crateName]}. If the API has changed
    since then, check the{" "}
    <a
      href={EXAMPLES_URLS[crateName]}
      target="_blank"
      rel="noopener noreferrer"
    >
      examples in the repo
    </a>{" "}
    for the latest usage.
  </Callout>
);
