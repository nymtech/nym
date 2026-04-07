import { Callout } from "nextra/components";

const CRATES_VERSION = "1.20.4";
const INSTALL_PATH = "/developers/rust/importing";

export const CratesPaused = () => (
  <Callout type="warning">
    <strong>Crate publication is paused.</strong> The crates.io release (v
    {CRATES_VERSION}) doesn't include the Stream module or other recent work.
    Publication resumes with the Lewes Protocol. Import from Git for now — see{" "}
    <a href={INSTALL_PATH}>Installation</a>.
  </Callout>
);
