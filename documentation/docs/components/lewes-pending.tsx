import { Callout } from "nextra/components";
import type { ReactNode } from "react";

/**
 * Centralised "Lewes Protocol release is coming" notice.
 *
 * When the Lewes Protocol ships, update the variant strings below in this one
 * file rather than searching the docs tree for individual callouts.
 *
 *   <LewesPending variant="latency" />
 *   <LewesPending variant="cryptography" />
 *   <LewesPending variant="acks" />
 */

type Variant = "latency" | "cryptography" | "acks";

interface LewesPendingProps {
  variant: Variant;
}

interface VariantEntry {
  type: "info" | "warning";
  body: ReactNode;
}

const VARIANTS: Record<Variant, VariantEntry> = {
  latency: {
    type: "info",
    body: "Updated latency measurements will be published after the Lewes Protocol release.",
  },
  cryptography: {
    type: "info",
    body: (
      <>
        Cryptographic details on this page will be updated for the Lewes
        Protocol release. For the current algorithm overview, see the{" "}
        <a
          href="https://nym.com/trust-center/cryptography"
          target="_blank"
          rel="noopener noreferrer"
        >
          Nym Trust Center: Cryptography
        </a>
        .
      </>
    ),
  },
  acks: {
    type: "warning",
    body: "The upcoming Lewes Protocol release will introduce changes to how acknowledgements are handled. The current hop-by-hop ACK mechanism described above may be revised as part of broader protocol improvements. Details will be documented here once the changes are finalised.",
  },
};

export const LewesPending = ({ variant }: LewesPendingProps) => {
  const { type, body } = VARIANTS[variant];
  return <Callout type={type}>{body}</Callout>;
};
