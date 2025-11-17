"use client";

import List from "@mui/material/List";
import ListItem from "@mui/material/ListItem";
import ListItemButton from "@mui/material/ListItemButton";
import ListItemIcon from "@mui/material/ListItemIcon";
import ListItemText from "@mui/material/ListItemText";
import Stack from "@mui/material/Stack";
import NextLink from "next/link";
import { usePathname } from "next/navigation";

import DoorSlidingOutlinedIcon from "@mui/icons-material/DoorSlidingOutlined";
import HubIcon from "@mui/icons-material/Hub";
import SettingsInputAntennaIcon from "@mui/icons-material/SettingsInputAntenna";
import ViewModuleIcon from "@mui/icons-material/ViewModule";
import WorkspacePremiumIcon from "@mui/icons-material/WorkspacePremium";

const mainListItemsAll = [
  { text: "Network Nodes", icon: <HubIcon />, url: "/nodes" },
  { text: "dVPN Gateways", icon: <DoorSlidingOutlinedIcon />, url: "/dvpn" },
  { text: "SOCKS5 NRs", icon: <SettingsInputAntennaIcon />, url: "/socks5" },
  {
    text: "zk-nym Signers",
    icon: <WorkspacePremiumIcon />,
    url: "/zk-nym-signers",
  },
  {
    text: "Nyx Chain Validators",
    icon: <ViewModuleIcon />,
    url: "/validators",
  },
];

const mainListItems = [mainListItemsAll[0], mainListItemsAll[1]];

export default function MenuContent() {
  const path = usePathname();
  return (
    <Stack sx={{ flexGrow: 1, p: 1, justifyContent: "space-between" }}>
      <List dense>
        {mainListItems.map((item, index) => (
          <ListItem
            key={`${index}-${item.url}`}
            disablePadding
            sx={{ display: "block" }}
          >
            <ListItemButton
              selected={path === item.url}
              component={NextLink}
              href={item.url}
            >
              <ListItemIcon>{item.icon}</ListItemIcon>
              <ListItemText primary={item.text} />
            </ListItemButton>
          </ListItem>
        ))}
      </List>
    </Stack>
  );
}
