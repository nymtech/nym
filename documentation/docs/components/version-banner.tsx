import { Callout } from "nextra/components";
import { NYM_SDK_VERSION } from "./versions";

const INSTALL_PATH = "/developers/rust/importing";

export const VersionBanner = () => (
  <Callout type="info">
    Code examples target <strong>v{NYM_SDK_VERSION}</strong> of the Nym crates
    on{" "}
    <a href="https://crates.io/crates/nym-sdk" target="_blank" rel="noopener noreferrer">
      crates.io
    </a>
    . See <a href={INSTALL_PATH}>Installation</a> for setup instructions.
  </Callout>
);
