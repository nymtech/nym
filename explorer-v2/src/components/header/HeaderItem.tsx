"use client";

import { Circle } from "@mui/icons-material";
import { Button, Stack } from "@mui/material";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { useEnvironment } from "../../providers/EnvironmentProvider";
import { getBasePathByEnv } from "../../../envs/config";
import type { MenuItem } from "./menuItems";

type HeaderItemProps = {
  menu: MenuItem;
};

const HeaderItem = ({ menu }: HeaderItemProps) => {
  const pathname = usePathname();
  const { environment } = useEnvironment();
  const basePath = getBasePathByEnv(environment || "mainnet");

  return (
    <Stack direction="row" gap={2} key={menu.id} alignItems="center">
      <Circle
        sx={{
          fontSize: 10,
          display: pathname.includes(menu.url) ? "block" : "none",
        }}
      />

      <Link href={`${basePath}${menu.url}`} passHref>
        <Button
          sx={{
            padding: 0,
          }}
        >
          {menu.title}
        </Button>
      </Link>
    </Stack>
  );
};

export default HeaderItem;
