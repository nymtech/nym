import { Callout } from "nextra/components";

const COMMIT_SHORT = "97068b2";
const COMMIT_FULL = "97068b2aa";
const EXAMPLES_URL =
  "https://github.com/nymtech/nym/tree/develop/sdk/rust/nym-sdk/examples";

export const CodeVerified = () => (
  <Callout type="info">
    Code verified against commit{" "}
    <a
      href={`https://github.com/nymtech/nym/commit/${COMMIT_FULL}`}
      target="_blank"
      rel="noopener noreferrer"
    >
      <code>{COMMIT_SHORT}</code>
    </a>
    . If the API has changed since then, check the{" "}
    <a href={EXAMPLES_URL} target="_blank" rel="noopener noreferrer">
      examples in the repo
    </a>{" "}
    for the latest usage.
  </Callout>
);
