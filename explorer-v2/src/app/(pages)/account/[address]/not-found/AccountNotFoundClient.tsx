"use client";

import ExplorerButtonGroup from "@/components/toggleButton/ToggleButton";
import { useEnvironment } from "@/providers/EnvironmentProvider";
import { getBasePathByEnv } from "../../../../../../envs/config";

interface AccountNotFoundClientProps {
  address: string;
}

export default function AccountNotFoundClient({
  address,
}: AccountNotFoundClientProps) {
  const { environment } = useEnvironment();
  const basePath = getBasePathByEnv(environment || "mainnet");

  return (
    <ExplorerButtonGroup
      onPage="Account"
      options={[
        {
          label: "Nym Node",
          isSelected: true,
          link: `${basePath}/account/${address}/not-found/`,
        },
        {
          label: "Account",
          isSelected: false,
          link: `${basePath}/account/${address}`,
        },
      ]}
    />
  );
}
