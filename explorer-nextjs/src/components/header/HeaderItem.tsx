"use client";

import { Circle } from "@mui/icons-material";
import { Button, Stack } from "@mui/material";
import Link from "next/link";
import { usePathname } from "next/navigation";
import type { MenuItem } from "./DesktopHeader";

type HeaderItemProps = {
  menu: MenuItem;
};
const HeaderItem = ({ menu }: HeaderItemProps) => {
  const pathname = usePathname();
  return (
    <Stack direction="row" gap={2} key={menu.id} alignItems="center">
      {pathname.includes(menu.url) && <Circle sx={{ fontSize: 10 }} />}
      <Link href={menu.url} passHref>
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
